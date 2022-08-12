use core::{
    future::Future,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

use avr_device::interrupt::CriticalSection;
use pin_utils::pin_mut;

use crate::{
    chip::RawRuntime,
    runtime::{Memory, Runtime},
};

#[doc(hidden)]
pub mod __private {
    #[no_mangle]
    pub static mut RUNTIME: crate::chip::RawRuntime = crate::chip::RawRuntime::uninit();

    #[inline(always)]
    pub unsafe fn get<'a, T: crate::runtime::Runtime>() -> &'a mut T {
        &mut *(RUNTIME.data as *mut T)
    }
}

static VTABLE: RawWakerVTable = {
    unsafe fn clone(_: *const ()) -> RawWaker {
        unimplemented!()
    }

    unsafe fn wake(_: *const ()) {
        crate::executor::wake()
    }

    unsafe fn wake_by_ref(_: *const ()) {
        crate::executor::wake()
    }

    unsafe fn drop(_: *const ()) {
        // no-op
    }

    RawWakerVTable::new(clone, wake, wake_by_ref, drop)
};

pub fn run<R, F, Fut>(main: F) -> !
where
    R: Runtime,
    Fut: Future<Output = ()>,
    F: FnOnce<R::Arguments, Output = Fut>,
{
    unsafe {
        ::core::arch::asm!("cli");
        let cs = CriticalSection::new();

        let mut mem = <<R as Runtime>::Memory as Memory>::alloc();

        let (mut runtime, args) = R::new(
            <<R as Runtime>::Memory as Memory>::from_ptr(&mut mem as *mut _),
            &cs,
        );

        self::__private::RUNTIME = RawRuntime::new(&runtime);
        let waker = Waker::from_raw(RawWaker::new(
            &self::__private::RUNTIME as *const _ as *const (),
            &VTABLE,
        ));
        let mut context = Context::from_waker(&waker);

        ::core::arch::asm!("sei");

        let task = core::ops::FnOnce::call_once(main, args);

        pin_mut!(task);

        loop {
            ::core::arch::asm!("cli");
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
#[inline(always)]
pub unsafe fn wake() {
    __avr_async_runtime_wake()
}

extern "Rust" {
    #[doc(hidden)]
    fn __avr_async_runtime_wake();
}
