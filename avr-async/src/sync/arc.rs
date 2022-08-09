use core::{borrow::Borrow, ops::Deref, ptr::NonNull};

use crate::slab::{Slab, SlabBox};

pub struct ArcSlab<T> {
    count: usize,
    value: T,
}

pub struct Arc<T>(NonNull<ArcSlab<T>>);

impl<T> Arc<T> {
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    #[inline(always)]
    pub fn new(slab: Slab<ArcSlab<T>>, value: T) -> Self {
        Self(unsafe {
            NonNull::new(SlabBox::leak(slab.get(ArcSlab { count: 1, value }))).unwrap_unchecked()
        })
    }

    const fn inner(&self) -> &ArcSlab<T> {
        unsafe { &*(self.0.as_ptr()) }
    }

    #[allow(clippy::mut_from_ref)]
    const fn inner_mut(&self) -> &mut ArcSlab<T> {
        unsafe { &mut *(self.0.as_ptr()) }
    }

    #[inline]
    const fn inc(&self) {
        self.inner_mut().count += 1;
    }

    #[inline]
    const fn value(&self) -> &T {
        &self.inner().value
    }

    #[inline]
    pub const fn into_raw(this: Self) -> *const T {
        let ptr = Self::as_ptr(&this);
        core::mem::forget(this);
        ptr
    }

    #[inline]
    pub const fn as_ptr(this: &Self) -> *const T {
        core::ptr::addr_of!(this.inner().value)
    }

    #[inline]
    pub const fn strong_count(this: &Self) -> usize {
        this.inner().count
    }

    fn dec(&self) {
        self.inner_mut().count -= 1;
        if self.inner().count == 0 {
            unsafe {
                let v = &mut *core::ptr::addr_of_mut!(self.inner_mut().value);
                core::ptr::drop_in_place(v);
            }
        }
    }
}

unsafe impl<T: Sync + Send> Send for Arc<T> {}
unsafe impl<T: Sync + Send> Sync for Arc<T> {}

impl<T> const Clone for Arc<T> {
    #[inline]
    fn clone(&self) -> Self {
        self.inc();
        Self(self.0)
    }
}

impl<T> Drop for Arc<T> {
    #[inline]
    fn drop(&mut self) {
        self.dec()
    }
}

impl<T> Deref for Arc<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.value()
    }
}

impl<T> AsRef<T> for Arc<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        self.value()
    }
}

impl<T> Borrow<T> for Arc<T> {
    #[inline]
    fn borrow(&self) -> &T {
        self.value()
    }
}

impl<T> Unpin for Arc<T> {}
