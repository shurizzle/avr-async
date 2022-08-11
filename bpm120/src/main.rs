#![no_std]
#![no_main]
// #![feature(abi_avr_interrupt, asm_experimental_arch)]
#![feature(asm_experimental_arch)]

use core::{future::Future, task::Poll};

use arduino_hal::{
    hal::port::{PB0, PD5},
    port::mode::Output,
};
use avr_async::{
    r#yield,
    slab::{Slab, SlabBox, Slabbed},
};
use heapless::Vec;
use panic_halt as _;

use avr_device::interrupt::{self, CriticalSection};

mod util;

pub type TickerSlab<const N: usize> = Vec<Option<u8>, N>;

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
    pub fn new(slab: Slab<Self>) -> Self {
        Self {
            half: false,
            changed: false,
            current: 0,
            snapshots: slab.get(Vec::new()),
        }
    }

    pub fn subscribe<'a>(&mut self) -> Option<TickerListener<'a>> {
        if matches!(self.snapshots.push(None), Ok(())) {
            Some(TickerListener(unsafe {
                &mut *((self.snapshots.as_mut_ptr() as *mut Option<u8>)
                    .add(self.snapshots.len() - 1))
            }))
        } else {
            None
        }
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

pub struct TickerListener<'a>(&'a mut Option<u8>);

impl<'a> TickerListener<'a> {
    pub fn next<'b>(&'b mut self) -> NextTick<'b, 'a> {
        NextTick { ticker: self }
    }
}

pub struct NextTick<'a, 'b> {
    ticker: &'a mut TickerListener<'b>,
}

impl<'a, 'b> Future for NextTick<'a, 'b> {
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

#[avr_async::slab]
struct GlobalSlab(pub Ticker<1>);

pub struct Runtime {
    tc1: arduino_hal::pac::TC1,
    pub led1: Option<arduino_hal::port::Pin<Output, PD5>>,
    pub led2: Option<arduino_hal::port::Pin<Output, PB0>>,
    cpu: arduino_hal::pac::CPU,
    ticker: Ticker<1>,
    ready: bool,
}

impl Runtime {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let peripherals = arduino_hal::Peripherals::take().unwrap();
        util::reset_irqs(&peripherals);
        let slab = GlobalSlab::take().unwrap();

        let (mut led1, mut led2) = {
            let pins = arduino_hal::pins!(peripherals);

            (pins.led_tx.into_output(), pins.led_rx.into_output())
        };

        led1.set_low();
        led2.set_low();

        Self {
            tc1: peripherals.TC1,
            led1: Some(led1),
            led2: Some(led2),
            cpu: peripherals.CPU,
            ticker: Ticker::new(slab.0),
            ready: false,
        }
    }

    #[inline]
    pub fn subscribe_ticker<'a>(&mut self) -> Option<TickerListener<'a>> {
        self.ticker.subscribe()
    }
}

impl avr_async::runtime::Ready for Runtime {
    #[inline]
    fn is_ready(&self, _: &CriticalSection) -> bool {
        self.ready
    }
}

impl avr_async::runtime::Runtime for Runtime {
    #[inline]
    fn init(&mut self, _: &CriticalSection) {
        unsafe { ::core::arch::asm!("sei") };

        // Set TIMER1_COMPA to 1/4s
        {
            self.tc1.tccr1a.write(|w| w.wgm1().bits(0));
            self.tc1.tccr1b.write(|w| w.cs1().bits(5).wgm1().bits(0b01));
            self.tc1.tcnt1.write(|w| unsafe { w.bits(0) });
            self.tc1.ocr1a.write(|w| unsafe { w.bits(3907) });
            self.tc1.tifr1.write(|w| w.tov1().bit(true));
            self.tc1.timsk1.write(|w| w.ocie1a().set_bit());
        }
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

async fn switch_leds(
    mut ticker: TickerListener<'_>,
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
pub unsafe extern "C" fn main() -> ! {
    ::core::arch::asm!("cli");

    let mut runtime = Runtime::new();

    let task1 = switch_leds(
        runtime.subscribe_ticker().unwrap(),
        runtime.led1.take().unwrap_unchecked(),
        runtime.led2.take().unwrap_unchecked(),
    );

    avr_async::executor::run(&mut runtime, avr_async::task_compose!(task1))
}
