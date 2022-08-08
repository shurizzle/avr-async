use core::{borrow::Borrow, mem::MaybeUninit, ops::Deref};

pub type ArcSlab<T> = MaybeUninit<(usize, T)>;

pub struct Arc<T: 'static>(*mut ArcSlab<T>);

impl<T> Arc<T> {
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    #[inline(always)]
    pub fn new(slab: *mut ArcSlab<T>, value: T) -> Self {
        unsafe { &mut *slab }.write((1, value));
        Self(slab)
    }

    const fn slab_ptr(&self) -> *mut (usize, T) {
        self.0 as *mut (usize, T)
    }

    #[allow(clippy::mut_from_ref)]
    #[inline(always)]
    const fn slab(&self) -> &mut (usize, T) {
        unsafe { &mut *self.slab_ptr() }
    }

    #[inline]
    const fn inc(&self) {
        self.slab().0 += 1;
    }

    #[inline]
    const fn counter_ptr(&self) -> *mut usize {
        unsafe { core::ptr::addr_of_mut!((*self.slab_ptr()).0) }
    }

    #[inline]
    const fn counter(&self) -> &usize {
        unsafe { &*self.counter_ptr() }
    }

    #[allow(clippy::mut_from_ref)]
    #[inline]
    const fn counter_mut(&self) -> &mut usize {
        unsafe { &mut *self.counter_ptr() }
    }

    #[inline]
    const fn value_ptr(&self) -> *mut T {
        unsafe { core::ptr::addr_of_mut!((*self.slab_ptr()).1) }
    }

    #[inline]
    const fn value(&self) -> &T {
        unsafe { &*self.value_ptr() }
    }

    #[inline]
    pub const fn into_raw(this: Self) -> *const T {
        let ptr = Self::as_ptr(&this);
        core::mem::forget(this);
        ptr
    }

    #[inline]
    pub const fn as_ptr(this: &Self) -> *const T {
        this.value_ptr()
    }

    #[inline]
    pub const fn strong_count(this: &Self) -> usize {
        *this.counter()
    }

    fn dec(&self) {
        *self.counter_mut() -= 1;
        if *self.counter() == 0 {
            unsafe {
                let v = &mut *core::ptr::addr_of_mut!((*self.slab_ptr()).1);
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
