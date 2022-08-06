#![no_std]
#![no_main]
#![feature(abi_avr_interrupt, asm_experimental_arch, const_pin, const_mut_refs)]

use core::{future::Future, mem::MaybeUninit, pin::Pin, task::Poll};

use panic_halt as _;

use avr_device::interrupt;

pub struct Ticker<'a>(&'a mut Option<u8>);

impl<'a> Ticker<'a> {
    #[allow(clippy::should_implement_trait)]
    #[inline]
    pub fn next<'b>(&'b mut self) -> NextTick<'b, 'a> {
        NextTick { ticker: self }
    }
}

pub struct NextTick<'a, 'b> {
    ticker: &'a mut Ticker<'b>,
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

pub struct TickState<'a, const N: usize> {
    half: bool,
    changed: bool,
    current: u8,
    snapshots: Pin<&'a mut [Option<u8>; N]>,
}

impl<'a, const N: usize> TickState<'a, N> {
    pub fn new(mut q: Pin<&'a mut [Option<u8>; N]>) -> (Self, [Ticker<'a>; N]) {
        for e in q.iter_mut() {
            *e = None;
        }

        Self::new_uninit(q)
    }

    pub fn new_uninit(mut q: Pin<&'a mut [Option<u8>; N]>) -> (Self, [Ticker<'a>; N]) {
        let tickers = {
            let mut tickers = MaybeUninit::<[Ticker<'a>; N]>::uninit();
            {
                let tickers = unsafe { &mut *tickers.as_mut_ptr() };
                for (i, t) in tickers.iter_mut().enumerate() {
                    t.0 = unsafe { &mut *q.as_mut_ptr().add(i) };
                }
            }
            unsafe { tickers.assume_init() }
        };

        (
            TickState {
                half: false,
                changed: false,
                current: 0,
                snapshots: q,
            },
            tickers,
        )
    }

    #[allow(clippy::cast_ref_to_mut)]
    #[inline(always)]
    pub fn ticker(&self, index: usize) -> Ticker<'a> {
        Ticker(unsafe { &mut *(&self.snapshots[index] as *const _ as *mut _) })
    }

    /// # Safety
    #[inline]
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

impl<'a, const N: usize> avr_async::runtime::State for TickState<'a, N> {
    fn snapshot(&mut self, _cs: &interrupt::CriticalSection) {
        if self.changed {
            self.snapshots.fill(Some(self.current));
            self.changed = false;
        } else {
            self.snapshots.fill(None);
        }
    }
}

type State<const N: usize> = TickState<'static, N>;

static mut __RUNTIME: *mut avr_async::runtime::DefaultRuntime<State<1>> = core::ptr::null_mut();

pub fn runtime() -> &'static mut avr_async::runtime::DefaultRuntime<State<1>> {
    unsafe { &mut *__RUNTIME }
}

fn reset_irqs(dp: &arduino_hal::Peripherals) {
    dp.EXINT.eimsk.reset(); // disable INTn
    dp.EXINT.pcmsk0.reset(); // disable PCINTn
    dp.TC0.timsk0.reset(); // disable TIMER0_* irqs
    dp.TC0.tccr0b.reset(); // disable TIMER0
    dp.TC1.timsk1.reset(); // disable TIMER1_* irqs
    dp.TC1.tccr1b.reset(); // disable TIMER1
    dp.TC3.timsk3.reset(); // disable TIMER3_* irqs
    dp.TC3.tccr3b.reset(); // disable TIMER3
    dp.TC4.timsk4.reset(); // disable TIMER4_* irqs
    dp.TC4.tccr4b.reset(); // disable TIMER4
    dp.USB_DEVICE.usbcon.reset(); // disable USB and interrupt
    dp.USB_DEVICE.udien.reset(); // disable USB interrupt
    dp.WDT.wdtcsr.reset(); // disable WDT
    dp.SPI.spcr.reset(); // disable SPI_STC
    dp.USART1.ucsr1b.reset(); // disable USART1_*
    dp.AC.acsr.reset(); // disable ANALOG_COMP
    dp.ADC.adcsra.reset(); // disable ADC
    dp.EEPROM.eecr.reset(); // disable EE_READY
    dp.TWI.twcr.reset(); // disable TWI
    dp.BOOT_LOAD.spmcsr.reset(); // disable SPM_READY
}

static mut Q: [Option<u8>; 1] = [None; 1];

#[arduino_hal::entry]
#[inline(always)]
fn main() -> ! {
    unsafe { ::core::arch::asm!("cli") };

    let dp = arduino_hal::Peripherals::take().unwrap();

    reset_irqs(&dp);

    unsafe { ::core::arch::asm!("sei") };

    let pins = arduino_hal::pins!(dp);

    let mut led1 = pins.led_tx.into_output();
    let mut led2 = pins.led_rx.into_output();

    led1.set_low();
    led2.set_low();

    let (state, [ticker]) = State::new_uninit(Pin::new(unsafe { &mut Q }));
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
