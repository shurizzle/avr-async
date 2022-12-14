use core::{cell::UnsafeCell, mem::MaybeUninit};

pub struct Queue<T, const N: usize> {
    bounds: Option<(usize, usize)>,
    buffer: [UnsafeCell<MaybeUninit<T>>; N],
}

impl<T, const N: usize> Queue<T, N> {
    const INIT: UnsafeCell<MaybeUninit<T>> = UnsafeCell::new(MaybeUninit::uninit());

    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            bounds: None,
            buffer: [Self::INIT; N],
        }
    }

    #[inline(always)]
    pub const fn capacity(&self) -> usize {
        N
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.bounds
            .as_ref()
            .map(|&(head, tail)| (tail.wrapping_sub(head).wrapping_add(N) % N) + 1)
            .unwrap_or(0)
    }

    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        matches!(self.bounds, None)
    }

    #[inline]
    pub fn is_full(&self) -> bool {
        self.bounds
            .as_ref()
            .map(|&(h, t)| h == Self::inc(t))
            .unwrap_or(false)
    }

    #[inline]
    pub fn enqueue(&mut self, val: T) -> Result<(), T> {
        unsafe { self.inner_enqueue(val) }
    }

    #[inline]
    pub fn dequeue(&mut self) -> Option<T> {
        unsafe { self.inner_dequeue() }
    }

    #[inline(always)]
    pub fn iter(&self) -> Iter<T, N> {
        self.into_iter()
    }

    #[inline(always)]
    pub fn iter_mut(&mut self) -> IterMut<T, N> {
        self.into_iter()
    }

    unsafe fn inner_enqueue(&mut self, val: T) -> Result<(), T> {
        match match self.bounds {
            Some((head, tail)) => {
                let next_tail = Self::inc(tail);

                if head == next_tail {
                    Err(())
                } else {
                    Ok((head, next_tail))
                }
            }
            None => Ok((0, 0)),
        } {
            Ok((head, tail)) => {
                self.bounds = Some((head, tail));
                self.buffer
                    .get_unchecked(tail)
                    .get()
                    .write(MaybeUninit::new(val));
                Ok(())
            }
            Err(()) => Err(val),
        }
    }

    unsafe fn inner_dequeue(&mut self) -> Option<T> {
        match self.bounds {
            Some((head, tail)) => {
                let v = (self.buffer.get_unchecked(head).get() as *const T).read();
                let len = tail.wrapping_sub(head).wrapping_add(N) % N;
                if len == 0 {
                    self.bounds = None;
                } else {
                    self.bounds = Some((Self::inc(head), tail));
                }
                Some(v)
            }
            None => None,
        }
    }

    #[inline(always)]
    const fn inc(val: usize) -> usize {
        (val + 1) % N
    }
}

impl<T, const N: usize> Default for Queue<T, N> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

pub struct Iter<'a, T, const N: usize> {
    q: &'a Queue<T, N>,
    len: usize,
    index: usize,
}

impl<'a, T, const N: usize> Iter<'a, T, N> {
    #[inline(always)]
    pub fn new(q: &'a Queue<T, N>) -> Self {
        Self {
            q,
            len: q.len(),
            index: 0,
        }
    }
}

impl<'a, T, const N: usize> Iterator for Iter<'a, T, N> {
    type Item = &'a T;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.len {
            None
        } else {
            let head = self.q.bounds.unwrap().0;
            let i = head.wrapping_add(self.index) % N;
            self.index += 1;
            Some(unsafe { &*(self.q.buffer.get_unchecked(i).get() as *const T) })
        }
    }
}

pub struct IterMut<'a, T, const N: usize> {
    q: &'a mut Queue<T, N>,
    len: usize,
    index: usize,
}

impl<'a, T, const N: usize> IterMut<'a, T, N> {
    #[inline(always)]
    pub fn new(q: &'a mut Queue<T, N>) -> Self {
        let len = q.len();

        Self { q, len, index: 0 }
    }
}

impl<'a, T, const N: usize> Iterator for IterMut<'a, T, N> {
    type Item = &'a mut T;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.len {
            None
        } else {
            let head = self.q.bounds.unwrap().0;
            let i = head.wrapping_add(self.index) % N;
            self.index += 1;
            Some(unsafe { &mut *self.q.buffer.get_unchecked_mut(i).get_mut().as_mut_ptr() })
        }
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a Queue<T, N> {
    type Item = &'a T;

    type IntoIter = Iter<'a, T, N>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        Iter::new(self)
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a mut Queue<T, N> {
    type Item = &'a mut T;

    type IntoIter = IterMut<'a, T, N>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        IterMut::new(self)
    }
}

// #[cfg(test)]
// mod tests {
//     #[test]
//     fn test() {
//         let mut q = super::Queue::<u32, 2>::new();
//
//         // half full
//         assert_eq!(q.enqueue(1), Ok(()));
//         assert_eq!(q.len(), 1);
//         assert!(!q.is_empty());
//         assert!(!q.is_full());
//
//         // dequeue existing value
//         assert_eq!(q.dequeue(), Some(1));
//         assert_eq!(q.len(), 0);
//         assert!(q.is_empty());
//         assert!(!q.is_full());
//
//         // enqueue
//         assert_eq!(q.enqueue(1), Ok(()));
//         assert_eq!(q.len(), 1);
//         assert!(!q.is_empty());
//         assert!(!q.is_full());
//
//         // enqueue
//         assert_eq!(q.enqueue(2), Ok(()));
//         assert_eq!(q.len(), 2);
//         assert!(!q.is_empty());
//         assert!(q.is_full());
//
//         // enqueue fail
//         assert_eq!(q.enqueue(3), Err(3));
//         assert_eq!(q.len(), 2);
//         assert!(!q.is_empty());
//         assert!(q.is_full());
//
//         // dequeue all value (2)
//         assert_eq!(q.dequeue(), Some(1));
//         assert_eq!(q.dequeue(), Some(2));
//         assert!(q.is_empty());
//         assert!(!q.is_full());
//
//         // dequeue on empty
//         assert_eq!(q.dequeue(), None);
//     }
// }
