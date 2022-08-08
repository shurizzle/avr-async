use core::{cell::UnsafeCell, mem::MaybeUninit};

use either::Either;

pub struct TryAcquireError;

#[derive(Debug)]
pub struct InnerSemaphore<const N: usize> {
    permits: usize,
    locking: usize,
    bounds: Option<(usize, usize)>,
    #[allow(clippy::type_complexity)]
    buffer: [UnsafeCell<MaybeUninit<Option<usize>>>; N],
}

impl<const N: usize> InnerSemaphore<N> {
    #[allow(clippy::declare_interior_mutable_const)]
    const INIT: UnsafeCell<MaybeUninit<Option<usize>>> = UnsafeCell::new(MaybeUninit::uninit());

    #[inline(always)]
    pub const fn new(permits: usize) -> Self {
        Self {
            permits,
            locking: 0,
            bounds: None,
            buffer: [Self::INIT; N],
        }
    }

    #[inline(always)]
    pub fn add_permits(&mut self, n: usize) {
        self.permits = self.permits.checked_add(n).unwrap();
        self.progress_queue();
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.bounds
            .as_ref()
            .map(|&(head, tail)| (tail.wrapping_sub(head).wrapping_add(N) % N) + 1)
            .unwrap_or(0)
    }

    pub fn try_acquire_enqueue(&mut self, perms: usize) -> Option<TryAcquireEnqueue<N>> {
        if let Ok(p) = unsafe { (*(self as *mut Self)).try_acquire(perms) } {
            return Some(TryAcquireEnqueue::from_permit(p));
        } else {
            self.try_enqueue(perms)
                .map(TryAcquireEnqueue::from_enqued_lock)
        }
    }

    pub fn try_acquire(&mut self, perms: usize) -> Result<SemaphorePermit<N>, TryAcquireError> {
        if perms > self.permits {
            panic!("Too many permits requested");
        }

        let avail = self.permits - self.locking;
        if avail >= perms {
            self.locking += perms;
            Ok(unsafe { SemaphorePermit::new(self, perms) })
        } else {
            Err(TryAcquireError)
        }
    }

    #[inline]
    fn try_enqueue(&mut self, perms: usize) -> Option<EnqueuedLock<N>> {
        unsafe { self.inner_enqueue(perms) }
    }

    unsafe fn inner_enqueue(&mut self, perms: usize) -> Option<EnqueuedLock<N>> {
        match match self.bounds {
            Some((head, tail)) => {
                let next_tail = Self::inc(tail);

                if head == next_tail {
                    None
                } else {
                    Some((head, next_tail))
                }
            }
            None => Some((0, 0)),
        } {
            Some((head, tail)) => {
                self.bounds = Some((head, tail));
                let ptr = self.buffer.get_unchecked(tail).get();
                ptr.write(MaybeUninit::new(Some(perms)));
                Some(EnqueuedLock::new(self, ptr as *mut Option<usize>, perms))
            }
            None => None,
        }
    }

    #[inline(always)]
    const fn inc(val: usize) -> usize {
        (val + 1) % N
    }

    fn signal_descriptor(&mut self) {
        loop {
            unsafe {
                match self.bounds {
                    Some((head, tail)) => {
                        if (&*(self.buffer.get_unchecked(head).get() as *const Option<usize>))
                            .is_none()
                        {
                            let new_len = tail.wrapping_sub(head).wrapping_add(N) % N;
                            if new_len == 0 {
                                self.bounds = None;
                            } else {
                                self.bounds = Some((Self::inc(head), tail));
                            }
                        } else {
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
    }

    fn release(&mut self, permits: usize) {
        self.locking -= permits;
        self.progress_queue();
    }

    fn progress_queue(&mut self) {
        let len = match self.len() {
            0 => return,
            x => x,
        };
        let head = unsafe { self.bounds.unwrap_unchecked() }.0;

        for i in 0..len {
            let idx = head.wrapping_add(i) % N;
            if let Some(task) =
                unsafe { &mut *(self.buffer.get_unchecked(idx).get() as *mut Option<usize>) }
                    .as_mut()
            {
                let remaining = self.permits - self.locking;
                if remaining <= *task {
                    *task -= remaining;
                    self.locking = self.permits;
                    return;
                } else {
                    self.locking += *task;
                    *task = 0;
                }
            }
        }
    }
}

#[derive(Debug)]
struct EnqueuedLock<'a, const N: usize> {
    q: &'a mut InnerSemaphore<N>,
    descriptor: &'a mut Option<usize>,
    permits: usize,
}

impl<'a, const N: usize> EnqueuedLock<'a, N> {
    #[inline(always)]
    unsafe fn new(
        q: &'a mut InnerSemaphore<N>,
        descriptor: *mut Option<usize>,
        permits: usize,
    ) -> Self {
        let descriptor = &mut *descriptor;

        Self {
            q,
            descriptor,
            permits,
        }
    }

    pub fn try_lock(self) -> Either<SemaphorePermit<'a, N>, Self> {
        if unsafe { self.descriptor.unwrap_unchecked() } == 0 {
            *self.descriptor = None;
            self.q.signal_descriptor();
            Either::Left(unsafe {
                SemaphorePermit::new(&mut *(self.q as *mut InnerSemaphore<N>), self.permits)
            })
        } else {
            Either::Right(self)
        }
    }
}

impl<'a, const N: usize> Drop for EnqueuedLock<'a, N> {
    fn drop(&mut self) {
        if let Some(d) = self.descriptor {
            let permits = self.permits - *d;
            *self.descriptor = None;
            self.q.signal_descriptor();
            self.q.release(permits);
        }
    }
}

#[derive(Debug)]
pub struct SemaphorePermit<'a, const N: usize> {
    q: &'a mut InnerSemaphore<N>,
    permits: usize,
}

impl<'a, const N: usize> SemaphorePermit<'a, N> {
    #[inline(always)]
    unsafe fn new(q: &'a mut InnerSemaphore<N>, total: usize) -> Self {
        Self { q, permits: total }
    }

    #[inline(always)]
    pub fn forget(self) {
        core::mem::forget(self);
    }
}

impl<'a, const N: usize> Drop for SemaphorePermit<'a, N> {
    #[inline(always)]
    fn drop(&mut self) {
        self.q.release(self.permits);
    }
}

#[derive(Debug)]
pub struct TryAcquireEnqueue<'a, const N: usize> {
    state: Either<SemaphorePermit<'a, N>, EnqueuedLock<'a, N>>,
}

impl<'a, const N: usize> TryAcquireEnqueue<'a, N> {
    #[inline(always)]
    const fn from_permit(p: SemaphorePermit<'a, N>) -> Self {
        Self {
            state: Either::Left(p),
        }
    }
    #[inline(always)]
    const fn from_enqued_lock(p: EnqueuedLock<'a, N>) -> Self {
        Self {
            state: Either::Right(p),
        }
    }

    pub fn try_lock(self) -> Either<SemaphorePermit<'a, N>, Self> {
        match self.state {
            Either::Left(p) => Either::Left(p),
            Either::Right(q) => match q.try_lock() {
                Either::Left(p) => Either::Left(p),
                Either::Right(q) => Either::Right(Self::from_enqued_lock(q)),
            },
        }
    }
}
