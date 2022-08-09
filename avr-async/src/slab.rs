use core::{
    borrow::{Borrow, BorrowMut},
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
};

pub trait Slabbed {
    type InnerType;
}

pub struct Slab<T: Slabbed> {
    mem: *mut MaybeUninit<T::InnerType>,
}

pub struct SlabBox<T> {
    mem: *mut T,
}

impl<T: Slabbed> Slab<T> {
    #[inline(always)]
    #[doc(hidden)]
    pub const unsafe fn new(mem: *mut MaybeUninit<T::InnerType>) -> Self {
        Self { mem }
    }

    #[inline(always)]
    pub fn get(self, value: T::InnerType) -> SlabBox<T::InnerType> {
        let mem = unsafe { &mut *self.mem };
        mem.write(value);
        unsafe { SlabBox::from_ptr(MaybeUninit::as_mut_ptr(mem)) }
    }
}

impl<T> SlabBox<T> {
    /// # Safety
    #[inline(always)]
    pub const unsafe fn from_ptr(mem: *mut T) -> Self {
        Self { mem }
    }

    #[inline(always)]
    pub const fn as_ptr(&self) -> *const T {
        self.mem
    }

    #[inline(always)]
    pub const fn as_ptr_mut(&mut self) -> *mut T {
        self.mem
    }

    #[inline(always)]
    pub const fn leak(this: Self) -> *mut T {
        let res = this.mem;
        core::mem::forget(this);
        res
    }
}

impl<T> Drop for SlabBox<T> {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe { core::ptr::drop_in_place(self.mem) }
    }
}

impl<T> const Deref for SlabBox<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mem }
    }
}

impl<T> const DerefMut for SlabBox<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mem }
    }
}

impl<T> const Borrow<T> for SlabBox<T> {
    #[inline(always)]
    fn borrow(&self) -> &T {
        Deref::deref(self)
    }
}

impl<T> const BorrowMut<T> for SlabBox<T> {
    #[inline(always)]
    fn borrow_mut(&mut self) -> &mut T {
        DerefMut::deref_mut(self)
    }
}

impl<T> const AsRef<T> for SlabBox<T> {
    #[inline(always)]
    fn as_ref(&self) -> &T {
        Deref::deref(self)
    }
}

impl<T> const AsMut<T> for SlabBox<T> {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut T {
        DerefMut::deref_mut(self)
    }
}

pub use avr_async_macros::slab as slab_internal;

#[macro_export]
macro_rules! slab {
    ($($tt:tt)+) => {
        $crate::slab::slab_internal!($crate, $($tt)+);
    };
}
