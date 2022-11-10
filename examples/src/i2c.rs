#![no_std]
#![no_main]
#![feature(abi_avr_interrupt, asm_experimental_arch)]

use core::mem::MaybeUninit;

use avr_hal_generic::clock::MHz16;
use panic_halt as _;

use avr_async::{
    main, r#yield,
    reexports::avr_hal_generic::clock,
    slab::Slab,
    twi::{TwoWireInterface1, TwoWireInterfaceDriver1},
};

mod util;

pub struct Runtime {
    twi: TwoWireInterfaceDriver1<MHz16>,
    cpu: avr_async::hal::pac::CPU,
    ready: bool,
}

impl avr_async::runtime::Ready for Runtime {
    fn is_ready(&self, _: &avr_async::CriticalSection) -> bool {
        self.ready
    }
}

impl avr_async::runtime::Runtime for Runtime {
    type Memory = Slab<TwoWireInterface1<MHz16>>;

    type Arguments = (TwoWireInterface1<MHz16>,);

    fn new(slab: Self::Memory, _: &avr_async::CriticalSection) -> (Self, Self::Arguments) {
        let peripherals = avr_async::Peripherals::take().unwrap();

        util::reset_irqs(&peripherals);

        let pins = avr_async::pins!(peripherals);

        let (twi, driver) = TwoWireInterface1::new(
            slab,
            avr_async::twi!(peripherals, pins, clock::MHz16, 50_000),
        );

        (
            Self {
                twi: driver,
                cpu: peripherals.CPU,
                ready: false,
            },
            (twi,),
        )
    }

    fn snapshot(&mut self, _: &avr_async::CriticalSection) {
        self.ready = false;
    }

    fn idle(&self) {
        self.cpu.smcr.write(|w| w.sm().idle().se().set_bit());
        unsafe { ::core::arch::asm!("sei\nsleep") };
    }

    fn wake(&mut self) {
        self.ready = true;
    }

    fn shutdown(&self) {
        self.cpu.smcr.write(|w| w.sm().pdown().se().set_bit());
        unsafe { ::core::arch::asm!("sei\nsleep") };
    }

    unsafe fn twi(&mut self, cs: &avr_async::CriticalSection) {
        self.twi.run(cs);
    }
}

fn blink1() {
    #[allow(clippy::uninit_assumed_init)]
    let peripheral = unsafe { MaybeUninit::<avr_async::Peripherals>::uninit().assume_init() };
    let pins = avr_async::pins!(peripheral);
    let mut led = pins.pc0.into_output();

    led.toggle();
}

fn blink2() {
    #[allow(clippy::uninit_assumed_init)]
    let peripheral = unsafe { MaybeUninit::<avr_async::Peripherals>::uninit().assume_init() };
    let pins = avr_async::pins!(peripheral);
    let mut led = pins.pc1.into_output();

    led.toggle();
}

#[main(runtime = Runtime)]
async fn main(mut twi: TwoWireInterface1<MHz16>) {
    let on = true;
    if twi
        .write(0x3c, &[0x40, 0xAE | if on { 1 } else { 0 }])
        .await
        .is_err()
    {
        blink1();
    }

    loop {
        r#yield().await;
    }
}
