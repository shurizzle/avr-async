use core::{
    cell::UnsafeCell,
    future::Future,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::Poll,
};

use crate::task;

use super::{queue::UniqueEnqueue, UniqueQueue};

pub struct Mutex<T, const N: usize> {
    lock: Option<usize>,
    value: UnsafeCell<T>,
    queue: UniqueQueue<usize, N>,
}

impl<T, const N: usize> Mutex<T, N> {
    #[inline(always)]
    pub fn new(initial: T) -> Self {
        Self {
            lock: None,
            value: UnsafeCell::new(initial),
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
        if self.lock.is_none() {
            self.lock = Some(tid);
            Some(())
        } else {
            None
        }
    }
}

unsafe impl<T: Send, const N: usize> Send for Mutex<T, N> {}
unsafe impl<T: Send, const N: usize> Sync for Mutex<T, N> {}

pub struct MutexGuard<'a, T, const N: usize> {
    mutex: &'a mut Mutex<T, N>,
}

impl<T, const N: usize> !Send for MutexGuard<'_, T, N> {}
unsafe impl<T: Sync, const N: usize> Sync for MutexGuard<'_, T, N> {}

impl<T, const N: usize> Deref for MutexGuard<'_, T, N> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.value.get() }
    }
}

impl<T, const N: usize> DerefMut for MutexGuard<'_, T, N> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.value.get() }
    }
}

impl<T, const N: usize> Drop for MutexGuard<'_, T, N> {
    #[inline]
    fn drop(&mut self) {
        self.mutex.lock = self.mutex.queue.try_dequeue();
    }
}

#[allow(clippy::type_complexity)]
pub struct WaitLock<'a, T, const N: usize> {
    state: Option<(
        &'a mut Mutex<T, N>,
        usize,
        Option<UniqueEnqueue<'a, usize, N>>,
    )>,
}

impl<'a, T, const N: usize> WaitLock<'a, T, N> {
    #[inline]
    pub fn new(mutex: &'a mut Mutex<T, N>, tid: usize) -> Self {
        if mutex.internal_try_lock(tid).is_some() {
            Self {
                state: Some((mutex, tid, None)),
            }
        } else {
            let queue = unsafe { &mut *(&mut mutex.queue as *mut UniqueQueue<usize, N>) };

            Self {
                state: Some((mutex, tid, Some(queue.enqueue(tid)))),
            }
        }
    }
}

impl<'a, T, const N: usize> Future for WaitLock<'a, T, N> {
    type Output = MutexGuard<'a, T, N>;

    fn poll(self: Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { Pin::get_unchecked_mut(self) };
        let (mutex, tid, mut state) = this.state.take().unwrap();

        let (state, res) = loop {
            match state {
                None => {
                    break match mutex.lock {
                        Some(x) if x == tid => (None, Poll::Ready(MutexGuard { mutex })),
                        _ => (Some((mutex, tid, None)), Poll::Pending),
                    }
                }
                Some(mut enqueue) => match (unsafe { Pin::new_unchecked(&mut enqueue) }).poll(cx) {
                    Poll::Ready(_) => {
                        state = None;
                    }
                    Poll::Pending => {
                        break (Some((mutex, tid, None)), Poll::Pending);
                    }
                },
            }
        };

        this.state = state;
        res
    }
}

impl<T: Default, const N: usize> Default for Mutex<T, N> {
    #[inline]
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T: Default, const N: usize> From<T> for Mutex<T, N> {
    #[inline]
    fn from(t: T) -> Self {
        Self::new(t)
    }
}
