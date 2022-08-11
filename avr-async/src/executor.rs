use core::{
    future::Future,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

use avr_device::interrupt::CriticalSection;
use pin_utils::pin_mut;

use crate::{chip::RawRuntime, runtime::Runtime};

#[no_mangle]
pub static mut RUNTIME: RawRuntime = RawRuntime::uninit();

static VTABLE: RawWakerVTable = {
    unsafe fn clone(_: *const ()) -> RawWaker {
        unimplemented!()
    }

    unsafe fn wake(p: *const ()) {
        wake_by_ref(p)
    }

    unsafe fn wake_by_ref(p: *const ()) {
        RawRuntime::from_ptr(p).wake()
    }

    unsafe fn drop(_: *const ()) {
        // no-op
    }

    RawWakerVTable::new(clone, wake, wake_by_ref, drop)
};

pub fn run<'a, R: Runtime>(runtime: &'a mut R, task: impl Future<Output = ()> + 'a) -> ! {
    unsafe { RUNTIME = RawRuntime::new(runtime) };
    let waker =
        unsafe { Waker::from_raw(RawWaker::new(&RUNTIME as *const _ as *const (), &VTABLE)) };
    let mut context = Context::from_waker(&waker);
    pin_mut!(task);

    runtime.init(unsafe { &CriticalSection::new() });

    loop {
        unsafe {
            ::core::arch::asm!("cli");
            let cs = CriticalSection::new();
            if runtime.is_ready(&cs) {
                runtime.snapshot(&cs);
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

#[doc(hidden)]
pub unsafe fn wake() {
    RUNTIME.wake()
}
