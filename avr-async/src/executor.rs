use core::{
    future::Future,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

use avr_device::interrupt::CriticalSection;
use pin_utils::pin_mut;

use crate::runtime::{Runtime, State};

static VTABLE: RawWakerVTable = {
    unsafe fn clone(_: *const ()) -> RawWaker {
        unimplemented!()
    }

    unsafe fn wake(_: *const ()) {}

    unsafe fn wake_by_ref(_: *const ()) {}

    unsafe fn drop(_: *const ()) {}

    RawWakerVTable::new(clone, wake, wake_by_ref, drop)
};

pub fn run<'a, S: State, R: Runtime<S>>(
    runtime: &'a mut R,
    task: impl Future<Output = ()> + 'a,
) -> ! {
    let waker = unsafe { Waker::from_raw(RawWaker::new(core::ptr::null(), &VTABLE)) };
    let mut context = Context::from_waker(&waker);
    pin_mut!(task);

    loop {
        unsafe {
            ::core::arch::asm!("cli");
            if runtime.is_ready() {
                runtime.snapshot(&CriticalSection::new());
                ::core::arch::asm!("sei");

                if let Poll::Ready(()) = task.as_mut().poll(&mut context) {
                    loop {
                        runtime.shutdown();
                    }
                }
            } else {
                ::core::arch::asm!("sei");
                runtime.idle();
            }
        }
    }
}
