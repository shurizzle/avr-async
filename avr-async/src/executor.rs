use core::{
    future::Future,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

use avr_device::interrupt::CriticalSection;
use pin_utils::pin_mut;

use crate::{runtime::Runtime, SyncUnsafeCell};

pub struct RawRuntime {
    data: *mut (),
    vtable: &'static Vtable,
}

unsafe impl Sync for RawRuntime {}

impl RawRuntime {
    #[inline]
    pub fn wake(&self) {
        unsafe { (self.vtable.wake)(self.data) }
    }

    #[inline]
    pub fn timer0_compa(&mut self, cs: &CriticalSection) {
        unsafe { (self.vtable.timer0_compa)(self.data, cs) }
    }

    #[doc(hidden)]
    #[inline]
    pub unsafe fn from_ptr<'a>(p: *const ()) -> &'a Self {
        &*(p as *const Self)
    }
}

pub(crate) struct Vtable {
    pub wake: unsafe fn(*mut ()),
    pub timer0_compa: unsafe fn(*mut (), &CriticalSection),
}

pub(crate) fn vtable<R: Runtime>() -> &'static Vtable {
    &Vtable {
        wake: _wake::<R>,
        timer0_compa: timer0_compa::<R>,
    }
}

unsafe fn _wake<R: Runtime>(ptr: *mut ()) {
    (*(ptr as *mut R)).wake()
}

unsafe fn timer0_compa<R: Runtime>(ptr: *mut (), cs: &CriticalSection) {
    (*(ptr as *mut R)).timer0_compa(cs)
}

fn raw_runtime<R: Runtime>(runtime: &R) -> RawRuntime {
    RawRuntime {
        data: runtime as *const R as *const () as *mut (),
        vtable: vtable::<R>(),
    }
}

#[allow(deref_nullptr)]
static RUNTIME: SyncUnsafeCell<Option<RawRuntime>> = SyncUnsafeCell::new(None);

#[doc(hidden)]
#[export_name = "__vector_17"]
pub unsafe extern "avr-interrupt" fn __vector17() {
    (*(RUNTIME.get()))
        .as_mut()
        .unwrap_unchecked()
        .timer0_compa(&CriticalSection::new())
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

pub fn run<'a, R: Runtime>(runtime: &'a mut R, task: impl Future<Output = ()> + 'a) -> ! {
    unsafe { core::ptr::write(RUNTIME.get(), Some(raw_runtime(runtime))) };
    let waker = unsafe {
        Waker::from_raw(RawWaker::new(
            (*RUNTIME.get()).as_ref().unwrap_unchecked() as *const RawRuntime as *const (),
            &VTABLE,
        ))
    };
    let mut context = Context::from_waker(&waker);
    pin_mut!(task);

    runtime.init(unsafe { &CriticalSection::new() });

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

#[doc(hidden)]
pub unsafe fn wake() {
    if let Some(runtime) = (*RUNTIME.get()).as_ref() {
        runtime.wake();
    }
}
