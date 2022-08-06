use core::{
    borrow::{Borrow, BorrowMut},
    ffi::c_void,
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    pin::Pin,
    ptr::NonNull,
};

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

pub struct Box<T: ?Sized>(*mut T);

impl<T: ?Sized> const Unpin for Box<T> {}

impl<T: ?Sized> Box<T> {
    /// # Safety
    /// This function is unsafe because improper use may lead to memory problems. For example, a double-free may occur if the function is called twice on the same raw pointer.
    #[inline(always)]
    pub const unsafe fn from_raw(raw: *mut T) -> Box<T> {
        Self(raw)
    }

    #[inline(always)]
    pub const fn leak<'a>(b: Self) -> &'a mut T {
        unsafe { &mut *Self::into_raw(b) }
    }

    #[inline(always)]
    pub const fn into_raw(b: Box<T>) -> *mut T {
        let res = b.0;
        core::mem::forget(b);
        res
    }

    #[inline(always)]
    pub const fn into_pin(boxed: Self) -> Pin<Self> {
        unsafe { Pin::new_unchecked(boxed) }
    }
}

/// # Do not try to allocate 0-sized types
impl<T> Box<T> {
    pub fn new(value: T) -> Self {
        let ptr = NonNull::new(unsafe { malloc(core::mem::size_of::<T>()) } as *mut T)
            .unwrap()
            .as_ptr();
        unsafe { core::ptr::copy(&value, ptr, 1) };
        core::mem::forget(value);
        Self(ptr)
    }

    #[inline(always)]
    pub fn pin(x: T) -> Pin<Box<T>> {
        Box::new(x).into()
    }

    #[inline(always)]
    pub fn new_uninit() -> Box<MaybeUninit<T>> {
        let ptr = NonNull::new(
            unsafe { malloc(core::mem::size_of::<MaybeUninit<T>>()) } as *mut MaybeUninit<T>
        )
        .unwrap()
        .as_ptr();
        Box(ptr)
    }

    #[inline]
    pub fn new_zeroed() -> Box<MaybeUninit<T>> {
        let mut b = Self::new_uninit();
        unsafe { b.as_mut_ptr().write_bytes(0u8, 1) };
        b
    }
}

impl<T> Box<MaybeUninit<T>> {
    /// Converts to `Box<T, A>`.
    ///
    /// # Safety
    ///
    /// As with [`MaybeUninit::assume_init`],
    /// it is up to the caller to guarantee that the value
    /// really is in an initialized state.
    /// Calling this when the content is not yet fully initialized
    /// causes immediate undefined behavior.
    #[inline]
    pub const unsafe fn assume_init(self) -> Box<T> {
        Box::from_raw(Box::into_raw(self) as *mut T)
    }
}

impl<T: ?Sized> Drop for Box<T> {
    #[inline]
    fn drop(&mut self) {
        unsafe { free(self.0 as *mut c_void) };
    }
}

impl<T: ?Sized> const Deref for Box<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        unsafe { &*self.0 }
    }
}

impl<T: ?Sized> const DerefMut for Box<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0 }
    }
}

impl<T: ?Sized> const AsRef<T> for Box<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        Deref::deref(self)
    }
}

impl<T: ?Sized> const AsMut<T> for Box<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        DerefMut::deref_mut(self)
    }
}

impl<T: ?Sized> const Borrow<T> for Box<T> {
    #[inline]
    fn borrow(&self) -> &T {
        Deref::deref(self)
    }
}

impl<T: ?Sized> const BorrowMut<T> for Box<T> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut T {
        DerefMut::deref_mut(self)
    }
}

#[allow(clippy::from_over_into)]
impl<T: ?Sized> const Into<Pin<Box<T>>> for Box<T> {
    #[inline]
    fn into(self) -> Pin<Box<T>> {
        Box::into_pin(self)
    }
}
