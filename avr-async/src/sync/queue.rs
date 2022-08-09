use core::{future::Future, pin::Pin, task::Poll};

pub struct Queue<T, const N: usize> {
    inner: crate::queue::Queue<T, N>,
}

impl<T, const N: usize> Queue<T, N> {
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            inner: crate::queue::Queue::new(),
        }
    }

    #[inline(always)]
    pub fn try_enqueue(&mut self, val: T) -> Result<(), T> {
        let signal = self.inner.is_empty();
        self.inner.enqueue(val).map(|x| {
            if signal {
                unsafe { crate::executor::wake() };
            }
            x
        })
    }

    #[inline(always)]
    pub fn try_dequeue(&mut self) -> Option<T> {
        let signal = self.inner.is_full();
        self.inner.dequeue().map(|x| {
            if signal {
                unsafe { crate::executor::wake() };
            }
            x
        })
    }

    #[inline(always)]
    pub fn enqueue(&mut self, val: T) -> Enqueue<T, N> {
        Enqueue::new(self, val)
    }

    #[inline(always)]
    pub fn dequeue(&mut self) -> Dequeue<T, N> {
        Dequeue::new(self)
    }
}

pub struct Enqueue<'a, T, const N: usize> {
    q: &'a mut Queue<T, N>,
    v: Option<T>,
}

impl<'a, T, const N: usize> Enqueue<'a, T, N> {
    #[inline(always)]
    pub fn new(q: &'a mut Queue<T, N>, val: T) -> Self {
        Self { q, v: Some(val) }
    }
}

impl<'a, T, const N: usize> Future for Enqueue<'a, T, N> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { Pin::get_unchecked_mut(self) };
        let signal = this.q.inner.is_empty();

        match this.q.try_enqueue(this.v.take().unwrap()) {
            Ok(()) => {
                if signal {
                    unsafe { crate::executor::wake() };
                }
                Poll::Ready(())
            }
            Err(v) => {
                this.v.replace(v);
                Poll::Pending
            }
        }
    }
}

pub struct Dequeue<'a, T, const N: usize> {
    q: Option<&'a mut Queue<T, N>>,
}

impl<'a, T, const N: usize> Dequeue<'a, T, N> {
    #[inline(always)]
    pub fn new(q: &'a mut Queue<T, N>) -> Self {
        Self { q: Some(q) }
    }
}

impl<'a, T, const N: usize> Future for Dequeue<'a, T, N> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, _cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { Pin::get_unchecked_mut(self) };
        let q = this.q.take().unwrap();
        let signal = q.inner.is_full();

        match q.try_dequeue() {
            Some(val) => {
                if signal {
                    unsafe { crate::executor::wake() };
                }
                Poll::Ready(val)
            }
            None => {
                this.q.replace(q);
                Poll::Pending
            }
        }
    }
}

impl<T, const N: usize> Default for Queue<T, N> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}
