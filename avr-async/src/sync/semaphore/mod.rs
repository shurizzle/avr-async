mod imp;

use core::{cell::UnsafeCell, future::Future, pin::Pin, task::Poll};

use either::Either;

use crate::runtime::Ready;

pub use self::imp::{SemaphorePermit, TryAcquireError};

#[derive(Debug)]
pub struct Semaphore<const N: usize> {
    pub(crate) inner: UnsafeCell<imp::InnerSemaphore<N>>,
}

impl<const N: usize> Semaphore<N> {
    #[inline(always)]
    pub const fn new(permits: usize) -> Self {
        Self {
            inner: UnsafeCell::new(imp::InnerSemaphore::new(permits)),
        }
    }

    #[inline(always)]
    pub fn add_permits(&self, n: usize) {
        self.inner().add_permits(n)
    }

    #[inline(always)]
    pub fn try_acquire_many(&self, n: usize) -> Result<SemaphorePermit<N>, TryAcquireError> {
        self.inner().try_acquire(n)
    }

    #[inline(always)]
    pub fn try_acquire(&self) -> Result<SemaphorePermit<N>, TryAcquireError> {
        self.try_acquire_many(1)
    }

    pub fn acquire_many(&self, n: usize) -> Acquire<N> {
        Acquire::new(self.inner.get(), n)
    }

    #[inline(always)]
    pub fn acquire(&self) -> Acquire<N> {
        self.acquire_many(1)
    }

    #[allow(clippy::mut_from_ref)]
    pub(crate) fn inner(&self) -> &mut imp::InnerSemaphore<N> {
        unsafe { &mut *(self.inner.get()) }
    }
}

impl<const N: usize> Ready for Semaphore<N> {
    #[inline]
    fn is_ready(&self, cs: &avr_device::interrupt::CriticalSection) -> bool {
        self.inner().is_ready(cs)
    }
}

pub struct Acquire<'a, const N: usize> {
    state: Option<Either<(&'a mut imp::InnerSemaphore<N>, usize), imp::TryAcquireEnqueue<'a, N>>>,
}

impl<'a, const N: usize> Acquire<'a, N> {
    #[inline(always)]
    fn new(q: *mut imp::InnerSemaphore<N>, n: usize) -> Self {
        Self {
            state: Some(Either::Left((unsafe { &mut *q }, n))),
        }
    }

    pub(crate) fn poll(
        &mut self,
        _cx: &mut core::task::Context<'_>,
    ) -> Poll<SemaphorePermit<'a, N>> {
        self.state = Some(loop {
            match unsafe { self.state.take().unwrap_unchecked() } {
                Either::Left((q, n)) => {
                    if let Some(p) =
                        unsafe { (*(q as *mut imp::InnerSemaphore<N>)).try_acquire_enqueue(n) }
                    {
                        self.state = Some(Either::Right(p));
                    } else {
                        break Either::Left((q, n));
                    }
                }
                Either::Right(p) => match p.try_lock() {
                    Either::Left(s) => return Poll::Ready(s),
                    Either::Right(p) => break Either::Right(p),
                },
            }
        });

        Poll::Pending
    }
}

impl<'a, const N: usize> Future for Acquire<'a, N> {
    type Output = SemaphorePermit<'a, N>;

    #[inline(always)]
    fn poll(self: Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        unsafe { Pin::get_unchecked_mut(self) }.poll(cx)
    }
}

// #[cfg(test)]
// mod tests {
//     use core::task::Poll;
//
//     fn unwrap_ready<T>(p: Poll<T>) -> T {
//         match p {
//             Poll::Ready(x) => x,
//             _ => panic!(),
//         }
//     }
//
//     #[test]
//     fn test() {
//         let s = super::Semaphore::<2>::new(2);
//
//         let mut fut1 = s.acquire_many(2);
//         assert!(matches!(fut1.poll(), Poll::Ready(_)));
//
//         let mut fut1 = {
//             let mut fut1 = s.acquire_many(2);
//             let permit = fut1.poll();
//             assert!(matches!(permit, Poll::Ready(_)));
//             let _permit = unwrap_ready(permit);
//
//             let mut fut1 = s.acquire();
//             assert!(matches!(fut1.poll(), Poll::Pending));
//             fut1
//         };
//         let permit1 = fut1.poll();
//         assert!(matches!(permit1, Poll::Ready(_)));
//         let _permit1 = unwrap_ready(permit1);
//
//         let mut fut2 = s.acquire();
//         let permit2 = fut2.poll();
//         assert!(matches!(permit2, Poll::Ready(_)));
//         let _permit2 = unwrap_ready(permit2);
//
//         let mut fut3 = s.acquire_many(2);
//         assert!(matches!(fut3.poll(), Poll::Pending));
//
//         s.add_permits(1);
//
//         assert!(matches!(fut3.poll(), Poll::Pending));
//
//         s.add_permits(1);
//
//         assert!(matches!(fut3.poll(), Poll::Ready(_)));
//     }
// }
