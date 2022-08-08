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
        self.inner.enqueue(val)
    }

    #[inline(always)]
    pub fn try_dequeue(&mut self) -> Option<T> {
        self.inner.dequeue()
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

        match this.q.try_enqueue(this.v.take().unwrap()) {
            Ok(()) => Poll::Ready(()),
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

        match q.try_dequeue() {
            Some(val) => Poll::Ready(val),
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

pub struct UniqueQueue<T: Eq, const N: usize> {
    inner: crate::queue::UniqueQueue<T, N>,
}

impl<T: Eq, const N: usize> UniqueQueue<T, N> {
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            inner: crate::queue::UniqueQueue::new(),
        }
    }

    #[inline(always)]
    pub fn try_enqueue(&mut self, val: T) -> Result<Option<T>, T> {
        self.inner.enqueue(val)
    }

    #[inline(always)]
    pub fn try_dequeue(&mut self) -> Option<T> {
        self.inner.dequeue()
    }

    #[inline(always)]
    pub fn enqueue(&mut self, val: T) -> UniqueEnqueue<T, N> {
        UniqueEnqueue::new(self, val)
    }

    #[inline(always)]
    pub fn dequeue(&mut self) -> UniqueDequeue<T, N> {
        UniqueDequeue::new(self)
    }
}

impl<T: Eq, const N: usize> Default for UniqueQueue<T, N> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

pub struct UniqueEnqueue<'a, T: Eq, const N: usize> {
    q: &'a mut UniqueQueue<T, N>,
    v: Option<T>,
}

impl<'a, T: Eq, const N: usize> UniqueEnqueue<'a, T, N> {
    #[inline(always)]
    pub fn new(q: &'a mut UniqueQueue<T, N>, val: T) -> Self {
        Self { q, v: Some(val) }
    }
}

impl<'a, T: Eq, const N: usize> Future for UniqueEnqueue<'a, T, N> {
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, _cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { Pin::get_unchecked_mut(self) };

        match this.q.try_enqueue(this.v.take().unwrap()) {
            Ok(res) => Poll::Ready(res),
            Err(v) => {
                this.v.replace(v);
                Poll::Pending
            }
        }
    }
}

pub struct UniqueDequeue<'a, T: Eq, const N: usize> {
    q: Option<&'a mut UniqueQueue<T, N>>,
}

impl<'a, T: Eq, const N: usize> UniqueDequeue<'a, T, N> {
    #[inline(always)]
    pub fn new(q: &'a mut UniqueQueue<T, N>) -> Self {
        Self { q: Some(q) }
    }
}

impl<'a, T: Eq, const N: usize> Future for UniqueDequeue<'a, T, N> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, _cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { Pin::get_unchecked_mut(self) };
        let q = this.q.take().unwrap();

        match q.try_dequeue() {
            Some(val) => Poll::Ready(val),
            None => {
                this.q.replace(q);
                Poll::Pending
            }
        }
    }
}
