#![no_std]
#![no_main]
#![feature(abi_avr_interrupt, asm_experimental_arch)]

use panic_halt as _;

use avr_async::{main, r#yield};

mod util;

pub struct Runtime {
    cpu: arduino_hal::pac::CPU,
    ready: bool,
}

impl avr_async::runtime::Ready for Runtime {
    fn is_ready(&self, _: &avr_async::CriticalSection) -> bool {
        self.ready
    }
}

impl avr_async::runtime::Runtime for Runtime {
    type Memory = ();

    type Arguments = ();

    fn new(_: Self::Memory, _: &avr_async::CriticalSection) -> (Self, Self::Arguments) {
        let peripherals = arduino_hal::Peripherals::take().unwrap();

        util::reset_irqs(&peripherals);

        (
            Self {
                cpu: peripherals.CPU,
                ready: false,
            },
            (),
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

    unsafe fn twi(&mut self, _: &avr_async::CriticalSection) {
        todo!()
    }
}

#[main(runtime = Runtime)]
async fn main() {
    loop {
        r#yield().await;
    }
}
