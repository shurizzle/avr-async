use core::{cell::UnsafeCell, mem::MaybeUninit};

pub struct UniqueQueue<T: Eq, const N: usize> {
    head: usize,
    tail: usize,
    buffer: [UnsafeCell<MaybeUninit<T>>; N],
}

impl<T: Eq, const N: usize> UniqueQueue<T, N> {
    const INIT: UnsafeCell<MaybeUninit<T>> = UnsafeCell::new(MaybeUninit::uninit());

    pub const fn new() -> Self {
        crate::sealed::greater_than_0::<N>();

        Self {
            head: 0,
            tail: 0,
            buffer: [Self::INIT; N],
        }
    }

    #[inline(always)]
    const fn inc(val: usize) -> usize {
        (val + 1) % N
    }

    #[inline(always)]
    pub const fn capacity(&self) -> usize {
        N - 1
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.tail.wrapping_sub(self.head).wrapping_add(N) % N
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.head == self.tail
    }

    #[inline]
    pub const fn is_full(&self) -> bool {
        Self::inc(self.tail) == self.head
    }

    #[inline]
    pub fn enqueue(&mut self, val: T) -> Result<Option<T>, T> {
        unsafe { self.inner_enqueue(val) }
    }

    #[inline]
    pub fn dequeue(&mut self) -> Option<T> {
        unsafe { self.inner_dequeue() }
    }

    unsafe fn check_unique(&self, val: &T) -> Result<(), ()> {
        let len = self.len();
        let mut idx = 0;
        while idx < len {
            let i = (self.head + idx) % N;
            idx += 1;
            let value = &*(self.buffer.get_unchecked(i).get() as *mut T);
            if value.eq(val) {
                return Err(());
            }
        }
        Ok(())
    }

    unsafe fn inner_enqueue(&mut self, val: T) -> Result<Option<T>, T> {
        if self.check_unique(&val).is_err() {
            Ok(Some(val))
        } else {
            let next = Self::inc(self.tail);

            if next != self.head {
                (self.buffer.get_unchecked(self.tail).get()).write(MaybeUninit::new(val));
                self.tail = next;
                Ok(None)
            } else {
                Err(val)
            }
        }
    }

    unsafe fn inner_dequeue(&mut self) -> Option<T> {
        if self.head == self.tail {
            None
        } else {
            let v = (self.buffer.get_unchecked(self.head).get() as *const T).read();
            self.head = Self::inc(self.head);
            Some(v)
        }
    }
}
