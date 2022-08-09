use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

static mut IN_RUNTIME: bool = false;
static mut TASKNO: usize = 0;
static mut CONTEXT_ACQUIRED: bool = false;

/// # Safety
/// Internal use only.
#[inline(always)]
pub unsafe fn is_in_runtime() -> bool {
    IN_RUNTIME
}

/// # Safety
/// Internal use only.
#[inline(always)]
pub unsafe fn ensure_runtime() {
    if !is_in_runtime() {
        panic!("You are trying to execute a task outside the task runtime");
    }
}

pub struct Task<F: Future<Output = ()>> {
    id: usize,
    future: Option<F>,
}

impl<F: Future<Output = ()>> Task<F> {
    #[inline(always)]
    pub fn new(id: usize, future: F) -> Self {
        Self {
            id,
            future: Some(future),
        }
    }

    pub fn poll(&mut self, cx: &mut Context) -> bool {
        unsafe { ensure_runtime() };

        if let Some(mut future) = self.future.take() {
            unsafe { TASKNO = self.id };
            let res = if matches!(
                Future::poll(unsafe { Pin::new_unchecked(&mut future) }, cx),
                Poll::Pending
            ) {
                self.future.replace(future);
                false
            } else {
                true
            };
            unsafe { TASKNO = 0 };
            res
        } else {
            true
        }
    }
}

#[inline]
pub fn current() -> usize {
    unsafe { ensure_runtime() };
    unsafe { TASKNO }
}

pub mod __private {
    pub use avr_async_macros::task_compose;
}

#[macro_export]
macro_rules! task_compose {
    ($($tt:tt)+) => {
        $crate::task::__private::task_compose!($crate, $($tt)+)
    };
}

pub struct TaskContext<F: Future<Output = ()>> {
    inner: F,
}

impl<F: Future<Output = ()>> TaskContext<F> {
    #[inline(always)]
    pub fn acquire(inner: F) -> Self {
        if unsafe { CONTEXT_ACQUIRED } {
            panic!("Context already acquired");
        } else {
            unsafe { CONTEXT_ACQUIRED = true };
            Self { inner }
        }
    }
}

impl<F: Future<Output = ()>> Future for TaskContext<F> {
    type Output = ();

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        unsafe {
            if is_in_runtime() {
                panic!("You are trying to execute task runtime in a task runtime");
            }

            IN_RUNTIME = true;
            let res = {
                let me = Pin::get_unchecked_mut(self);
                let inner = Pin::new_unchecked(&mut me.inner);
                Future::poll(inner, cx)
            };
            IN_RUNTIME = false;
            res
        }
    }
}
