#![no_std]
#![no_main]
// #![feature(abi_avr_interrupt, asm_experimental_arch)]
#![feature(asm_experimental_arch)]

use core::{future::Future, mem::MaybeUninit, task::Poll};

use arduino_hal::{
    hal::port::{PB0, PD5},
    port::mode::Output,
};
use avr_async::{
    r#yield,
    slab::{Slab, SlabBox, Slabbed},
};
use panic_halt as _;

use avr_device::interrupt::{self, CriticalSection};

mod util;

pub type TickerSlab<const N: usize> = [Option<u8>; N];

pub struct Ticker<const N: usize> {
    half: bool,
    changed: bool,
    current: u8,
    snapshots: SlabBox<TickerSlab<N>>,
}

impl<const N: usize> Slabbed for Ticker<N> {
    type InnerType = TickerSlab<N>;
}

impl<const N: usize> Ticker<N> {
    #[inline(always)]
    pub fn new(slab: Slab<Self>) -> (Self, [TickerListener; N]) {
        let mut snapshots = slab.get([None; N]);

        let listeners = unsafe {
            let mut listeners = MaybeUninit::<[TickerListener; N]>::uninit();
            for i in 0..N {
                *((listeners.as_mut_ptr() as *mut TickerListener).add(i)) =
                    TickerListener(&mut *(snapshots.as_mut_ptr() as *mut Option<u8>));
            }
            listeners.assume_init()
        };

        (
            Self {
                half: false,
                changed: false,
                current: 0,
                snapshots,
            },
            listeners,
        )
    }

    pub fn snapshot(&mut self, _cs: &interrupt::CriticalSection) {
        if self.changed {
            self.snapshots.fill(Some(self.current));
            self.changed = false;
        } else {
            self.snapshots.fill(None);
        }
    }

    #[doc(hidden)]
    pub unsafe fn tick(&mut self) -> bool {
        if self.half {
            self.half = false;
            self.changed = true;
            self.current = (self.current + 1) % 4;
        } else {
            self.half = true;
        }
        self.changed
    }
}

pub struct TickerListener(&'static mut Option<u8>);

impl TickerListener {
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> NextTick {
        NextTick { ticker: self }
    }
}

pub struct NextTick<'a> {
    ticker: &'a mut TickerListener,
}

impl<'a> Future for NextTick<'a> {
    type Output = u8;

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        _cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        self.ticker
            .0
            .take()
            .map(Poll::Ready)
            .unwrap_or(Poll::Pending)
    }
}

pub struct Runtime<const N: usize> {
    cpu: arduino_hal::pac::CPU,
    ticker: Ticker<N>,
    ready: bool,
}

impl<const N: usize> avr_async::runtime::Ready for Runtime<N> {
    #[inline]
    fn is_ready(&self, _: &CriticalSection) -> bool {
        self.ready
    }
}

impl<const N: usize> avr_async::runtime::Runtime for Runtime<N> {
    type Memory = Slab<Ticker<N>>;

    type Arguments = (
        [TickerListener; N],
        arduino_hal::port::Pin<Output, PD5>,
        arduino_hal::port::Pin<Output, PB0>,
    );

    fn new(mem: Self::Memory, _: &CriticalSection) -> (Self, Self::Arguments) {
        let peripherals = arduino_hal::Peripherals::take().unwrap();

        util::reset_irqs(&peripherals);

        // Set TIMER1_COMPA to 1/4s
        {
            peripherals.TC1.tccr1a.write(|w| w.wgm1().bits(0));
            peripherals
                .TC1
                .tccr1b
                .write(|w| w.cs1().bits(5).wgm1().bits(0b01));
            peripherals.TC1.tcnt1.write(|w| unsafe { w.bits(0) });
            peripherals.TC1.ocr1a.write(|w| unsafe { w.bits(3907) });
            peripherals.TC1.tifr1.write(|w| w.tov1().bit(true));
            peripherals.TC1.timsk1.write(|w| w.ocie1a().set_bit());
        }

        let (mut led1, mut led2) = {
            let pins = arduino_hal::pins!(peripherals);

            (pins.led_tx.into_output(), pins.led_rx.into_output())
        };

        led1.set_low();
        led2.set_low();

        let (ticker, listeners) = Ticker::new(mem);

        (
            Self {
                cpu: peripherals.CPU,
                ticker,
                ready: false,
            },
            (listeners, led1, led2),
        )
    }

    #[inline]
    fn snapshot(&mut self, cs: &CriticalSection) {
        self.ready = false;
        self.ticker.snapshot(cs)
    }

    #[inline]
    fn idle(&self) {
        self.cpu.smcr.write(|w| w.sm().idle().se().set_bit());
        unsafe { ::core::arch::asm!("sleep") };
    }

    #[inline]
    fn wake(&mut self) {
        unsafe { core::ptr::write_volatile(&mut self.ready, true) }
    }

    #[inline]
    fn shutdown(&self) {
        self.cpu.smcr.write(|w| w.sm().pdown().se().set_bit());
        unsafe { ::core::arch::asm!("sleep") };
    }

    #[inline]
    unsafe fn timer1_compa(&mut self, _cs: &CriticalSection) {
        if self.ticker.tick() {
            self.wake()
        }
    }
}

async fn main(
    [mut ticker]: [TickerListener; 1],
    mut led1: arduino_hal::port::Pin<Output, PD5>,
    mut led2: arduino_hal::port::Pin<Output, PB0>,
) {
    let mut status = false;
    r#yield().await;

    loop {
        if ticker.next().await == 0 {
            led1.set_low();
            led2.set_low();
        } else if status {
            led1.set_low();
            led2.set_high();
            status = false;
        } else {
            led1.set_high();
            led2.set_low();
            status = true;
        }
    }
}

#[doc(hidden)]
#[export_name = "main"]
pub unsafe extern "C" fn __avr_async_main() -> ! {
    avr_async::executor::run::<Runtime<1>, _, _>(main)
}
