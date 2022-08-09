#![no_std]
#![no_main]
#![feature(abi_avr_interrupt, asm_experimental_arch)]

use core::{future::Future, task::Poll};

use avr_async::slab::{Slab, SlabBox, Slabbed};
use heapless::Vec;
use panic_halt as _;

use avr_device::interrupt;

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

impl<const N: usize> avr_async::runtime::State for Ticker<N> {
    fn snapshot(&mut self, _cs: &interrupt::CriticalSection) {
        if self.changed {
            self.snapshots.fill(Some(self.current));
            self.changed = false;
        } else {
            self.snapshots.fill(None);
        }
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

type State = Ticker<1>;

static mut __RUNTIME: *mut avr_async::runtime::DefaultRuntime<State> = core::ptr::null_mut();

pub fn runtime() -> &'static mut avr_async::runtime::DefaultRuntime<State> {
    unsafe { &mut *__RUNTIME }
}

avr_async::slab!(GlobalSlab { pub ticker: Ticker<1> });

#[arduino_hal::entry]
#[inline(always)]
fn main() -> ! {
    unsafe { ::core::arch::asm!("cli") };

    let dp = arduino_hal::Peripherals::take().unwrap();

    util::reset_irqs(&dp);

    unsafe { ::core::arch::asm!("sei") };

    let pins = arduino_hal::pins!(dp);

    let mut led1 = pins.led_tx.into_output();
    let mut led2 = pins.led_rx.into_output();

    led1.set_low();
    led2.set_low();

    let state = Ticker::new(GlobalSlab::take().unwrap().ticker);
    let mut rtm = avr_async::runtime::DefaultRuntime::new(state, dp.CPU);
    unsafe { __RUNTIME = &mut rtm as *mut _ };

    // Set TIMER1_COMPA to 1/4s
    {
        let tc1 = dp.TC1;
        tc1.tccr1a.write(|w| w.wgm1().bits(0));
        tc1.tccr1b.write(|w| w.cs1().bits(5).wgm1().bits(0b01));
        tc1.tcnt1.write(|w| unsafe { w.bits(0) });
        tc1.ocr1a.write(|w| unsafe { w.bits(3907) });
        tc1.tifr1.write(|w| w.tov1().bit(true));
        tc1.timsk1.write(|w| w.ocie1a().set_bit());
    }

    let ticker = {
        use avr_async::runtime::Runtime;
        unsafe { runtime().state_mut() }.subscribe().unwrap()
    };

    avr_async::executor::run(
        &mut rtm,
        avr_async::task_compose!(async move {
            let mut ticker = ticker;
            let mut status = false;

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
        }),
    )
}

#[doc(hidden)]
#[export_name = "__vector_17"]
pub unsafe extern "avr-interrupt" fn timer() {
    use avr_async::runtime::Runtime;
    runtime().modify(|state| state.tick());
}
