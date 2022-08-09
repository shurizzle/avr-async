use core::{
    borrow::{Borrow, BorrowMut},
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
};

pub struct Slab<T> {
    mem: *mut MaybeUninit<T>,
}

pub struct SlabBox<T> {
    mem: *mut T,
}

impl<T> Slab<T> {
    #[inline(always)]
    #[doc(hidden)]
    pub const unsafe fn new(mem: *mut MaybeUninit<T>) -> Self {
        Self { mem }
    }

    #[inline(always)]
    pub fn get(self, value: T) -> SlabBox<T> {
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

#[macro_export]
macro_rules! slab {
    ($t:ty) => {{
        static mut SLAB: MaybeUninit<$t> = MaybeUninit::uninit();
        unsafe { Slab::<$t>::new((&mut SLAB) as *mut MaybeUninit<$t>) }
    }};
}

#[macro_export]
macro_rules! slab1 {
    () => {{
        unsafe {
            $crate::slab::Slab::new((|x| {
                static mut SLAB: ::core::mem::MaybeUninit<[u8; ::core::mem::size_of_val_raw(x)]> =
                    ::core::mem::MaybeUninit::uninit();
                $crate::slab::__private_slab_cast_type(
                    &mut SLAB
                        as *mut ::core::mem::MaybeUninit<[u8; ::core::mem::size_of_val_raw(x)]>,
                )
            })($crate::slab::__private_slab_get_type()))
        }
    }};
}

/// # Safety
#[allow(clippy::zero_ptr)]
#[inline(always)]
pub const unsafe fn __private_slab_get_type<T>() -> *mut T {
    0 as *mut T
}

/// # Safety
#[inline(always)]
pub const unsafe fn __private_slab_cast_type<T, const N: usize>(
    mem: *mut MaybeUninit<[u8; N]>,
) -> *mut MaybeUninit<T> {
    mem as *mut MaybeUninit<T>
}
