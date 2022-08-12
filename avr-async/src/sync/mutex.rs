use core::{
    cell::UnsafeCell,
    future::Future,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::Poll,
};

use crate::runtime::Ready;

use super::semaphore::{Acquire, Semaphore};

pub struct TryLockError;

pub struct Mutex<T, const N: usize> {
    lock: Semaphore<N>,
    value: UnsafeCell<T>,
}

impl<T, const N: usize> Mutex<T, N> {
    #[inline(always)]
    pub const fn new(initial: T) -> Self {
        Self {
            lock: Semaphore::new(1),
            value: UnsafeCell::new(initial),
        }
    }

    #[inline(always)]
    pub fn lock(&mut self) -> Lock<T, N> {
        Lock::new(self)
    }

    pub fn try_lock(&mut self) -> Result<MutexGuard<T, N>, TryLockError> {
        unsafe { (*(self as *mut Self)).lock.try_acquire() }
            .map_err(|_| TryLockError)
            .map(|x| {
                core::mem::forget(x);
                MutexGuard { mutex: self }
            })
    }
}

impl<T, const N: usize> Ready for Mutex<T, N> {
    #[inline]
    fn is_ready(&self, cs: &avr_device::interrupt::CriticalSection) -> bool {
        self.lock.is_ready(cs)
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
        self.mutex.lock.inner().release(1);
    }
}

impl<'a, T, const N: usize> MutexGuard<'a, T, N> {
    pub fn map<U, F>(mut this: Self, f: F) -> MappedMutexGuard<'a, U, N>
    where
        F: FnOnce(&mut T) -> &mut U,
    {
        let data = f(&mut *this) as *mut U;
        let s = &mut this.mutex.lock as *mut Semaphore<N>;
        core::mem::forget(this);
        MappedMutexGuard {
            s: unsafe { &mut *s },
            data,
            marker: core::marker::PhantomData,
        }
    }
}

pub struct MappedMutexGuard<'a, T, const N: usize> {
    s: &'a mut Semaphore<N>,
    data: *mut T,
    marker: core::marker::PhantomData<&'a mut T>,
}

unsafe impl<'a, T, const N: usize> Sync for MappedMutexGuard<'a, T, N> where T: Sync + 'a {}
unsafe impl<'a, T, const N: usize> Send for MappedMutexGuard<'a, T, N> where T: Send + 'a {}

impl<'a, T, const N: usize> Deref for MappedMutexGuard<'a, T, N> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}

impl<'a, T, const N: usize> DerefMut for MappedMutexGuard<'a, T, N> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.data }
    }
}

impl<'a, T, const N: usize> Drop for MappedMutexGuard<'a, T, N> {
    #[inline]
    fn drop(&mut self) {
        self.s.inner().release(1);
    }
}

#[allow(clippy::type_complexity)]
pub struct Lock<'a, T, const N: usize> {
    state: Option<(&'a mut Mutex<T, N>, Acquire<'a, N>)>,
}

impl<'a, T, const N: usize> Lock<'a, T, N> {
    #[inline]
    pub fn new(mutex: &'a mut Mutex<T, N>) -> Self {
        let acquire = unsafe { (*(mutex as *mut Mutex<T, N>)).lock.acquire() };
        Self {
            state: Some((mutex, acquire)),
        }
    }

    fn poll(&mut self, cx: &mut core::task::Context<'_>) -> Poll<MutexGuard<'a, T, N>> {
        let (mutex, mut acquire) = unsafe { self.state.take().unwrap_unchecked() };

        match acquire.poll(cx) {
            Poll::Pending => {
                self.state = Some((mutex, acquire));
                Poll::Pending
            }
            Poll::Ready(perm) => {
                core::mem::forget(perm);
                Poll::Ready(MutexGuard { mutex })
            }
        }
    }
}

impl<'a, T, const N: usize> Future for Lock<'a, T, N> {
    type Output = MutexGuard<'a, T, N>;

    #[inline(always)]
    fn poll(self: Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        unsafe { Pin::get_unchecked_mut(self) }.poll(cx)
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
