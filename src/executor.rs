use core::{
    future::Future,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

use avr_device::interrupt;
use pin_utils::pin_mut;

use crate::runtime::{Runtime, State};

static VTABLE: RawWakerVTable = {
    unsafe fn clone(p: *const ()) -> RawWaker {
        RawWaker::new(p, &VTABLE)
    }

    unsafe fn wake(p: *const ()) {
        wake_by_ref(p)
    }

    unsafe fn wake_by_ref(p: *const ()) {
        core::ptr::write_volatile(p as *mut bool, true);
    }

    unsafe fn drop(_: *const ()) {}

    RawWakerVTable::new(clone, wake, wake_by_ref, drop)
};

pub fn run<'a, S: State>(runtime: &'a mut Runtime<S>, task: impl Future<Output = ()> + 'a) -> ! {
    let waker = unsafe {
        Waker::from_raw(RawWaker::new(
            core::mem::transmute(&runtime.ready as *const _),
            &VTABLE,
        ))
    };
    let mut context = Context::from_waker(&waker);
    pin_mut!(task);

    loop {
        while runtime.is_ready() {
            interrupt::free(|cs| {
                runtime.snapshot(cs);
            });

            if let Poll::Ready(()) = task.as_mut().poll(&mut context) {
                // TODO: shutdown
            }
        }

        // Set registers to enter idle mode on sleep
        // dp.CPU.smcr.write(|w| w.sm().idle().se().set_bit());
        // unsafe { ::core::arch::asm!("sleep") };

        // TODO: suspend
    }
}
