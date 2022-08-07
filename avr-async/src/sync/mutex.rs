use core::{
    future::Future,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::Poll,
};

use either::Either;

use crate::{queue::UniqueQueue, task};

pub struct Mutex<T, const N: usize> {
    locking: Option<usize>,
    value: T,
    // TODO: Use async unique queue instead
    queue: UniqueQueue<usize, N>,
}

impl<T, const N: usize> Mutex<T, N> {
    #[inline(always)]
    pub const fn new(initial: T) -> Self {
        crate::sealed::greater_than_0::<N>();

        Self {
            locking: None,
            value: initial,
            queue: UniqueQueue::new(),
        }
    }

    #[inline(always)]
    pub fn lock(&mut self) -> WaitLock<T, N> {
        WaitLock::new(self, task::current())
    }

    #[inline]
    pub fn try_lock(&mut self) -> Option<MutexGuard<T, N>> {
        self.internal_try_lock(task::current())
            .map(|_| MutexGuard { mutex: self })
    }

    fn internal_try_lock(&mut self, tid: usize) -> Option<()> {
        if self.locking.is_none() {
            self.locking = Some(tid);
            Some(())
        } else {
            None
        }
    }
}

pub struct MutexGuard<'a, T, const N: usize> {
    mutex: &'a mut Mutex<T, N>,
}

impl<'a, T, const N: usize> Deref for MutexGuard<'a, T, N> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.mutex.value
    }
}

impl<'a, T, const N: usize> DerefMut for MutexGuard<'a, T, N> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.mutex.value
    }
}

impl<'a, T, const N: usize> Drop for MutexGuard<'a, T, N> {
    #[inline]
    fn drop(&mut self) {
        self.mutex.locking = self.mutex.queue.dequeue();
    }
}

#[allow(clippy::type_complexity)]
pub struct WaitLock<'a, T, const N: usize> {
    state: Option<Either<(&'a mut Mutex<T, N>, usize), EnqueueLock<'a, T, N>>>,
}

impl<'a, T, const N: usize> WaitLock<'a, T, N> {
    #[inline]
    pub fn new(mutex: &'a mut Mutex<T, N>, tid: usize) -> Self {
        if mutex.internal_try_lock(tid).is_some() {
            Self {
                state: Some(Either::Left((mutex, tid))),
            }
        } else {
            Self {
                state: Some(Either::Right(EnqueueLock::new(mutex, tid))),
            }
        }
    }
}

impl<'a, T, const N: usize> Future for WaitLock<'a, T, N> {
    type Output = MutexGuard<'a, T, N>;

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        let this = Pin::get_mut(self);
        let mut state = this.state.take().unwrap();

        let (state, res) = loop {
            match state {
                Either::Left((mutex, tid)) => {
                    break match mutex.locking {
                        Some(x) if x == tid => (None, Poll::Ready(MutexGuard { mutex })),
                        _ => (Some(Either::Left((mutex, tid))), Poll::Pending),
                    }
                }
                Either::Right(mut enqueue) => {
                    match (unsafe { Pin::new_unchecked(&mut enqueue) }).poll(cx) {
                        Poll::Ready(mutex) => {
                            state = Either::Left(mutex);
                        }
                        Poll::Pending => {
                            break (Some(Either::Right(enqueue)), Poll::Pending);
                        }
                    }
                }
            }
        };

        this.state = state;
        res
    }
}

struct EnqueueLock<'a, T, const N: usize> {
    mutex: Option<&'a mut Mutex<T, N>>,
    tid: usize,
}

impl<'a, T, const N: usize> EnqueueLock<'a, T, N> {
    #[inline(always)]
    pub fn new(mutex: &'a mut Mutex<T, N>, tid: usize) -> Self {
        Self {
            mutex: Some(mutex),
            tid,
        }
    }
}

impl<'a, T, const N: usize> Future for EnqueueLock<'a, T, N> {
    type Output = (&'a mut Mutex<T, N>, usize);

    #[inline]
    fn poll(
        self: Pin<&mut Self>,
        _cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        let this = Pin::get_mut(self);
        let mutex = this.mutex.take().unwrap();
        if mutex.queue.enqueue(this.tid).is_ok() {
            Poll::Ready((mutex, this.tid))
        } else {
            this.mutex.replace(mutex);
            Poll::Pending
        }
    }
}
