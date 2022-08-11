use core::{
    future::Future,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

use avr_device::interrupt::CriticalSection;
use pin_utils::pin_mut;

use crate::{chip::RawRuntime, runtime::Runtime};

pub mod __private {
    #[no_mangle]
    pub static mut RUNTIME: crate::chip::RawRuntime = crate::chip::RawRuntime::uninit();
}

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

pub fn run<'a, R, Fut, F>(runtime: &'a mut R, main: F) -> !
where
    R: Runtime,
    Fut: Future<Output = ()> + 'a,
    F: FnOnce<R::Result, Output = Fut>,
{
    unsafe { self::__private::RUNTIME = RawRuntime::new(runtime) };
    let waker = unsafe {
        Waker::from_raw(RawWaker::new(
            &self::__private::RUNTIME as *const _ as *const (),
            &VTABLE,
        ))
    };
    let mut context = Context::from_waker(&waker);
    let task = {
        let args = runtime.init(unsafe { &CriticalSection::new() });
        unsafe { ::core::arch::asm!("sei") };
        core::ops::FnOnce::call_once(main, args)
    };

    pin_mut!(task);

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
    self::__private::RUNTIME.wake()
}
