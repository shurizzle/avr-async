use core::mem::MaybeUninit;

use avr_device::interrupt::CriticalSection;

pub trait Ready {
    fn is_ready(&self, cs: &CriticalSection) -> bool;
}

pub trait Memory: Sized {
    type Slab;

    fn alloc() -> Self::Slab;

    /// # Safety
    /// Dereferencing a pointer is always unsafe.
    unsafe fn from_ptr(mem: *mut Self::Slab) -> Self;
}

impl<T: Slabbed> Memory for Slab<T> {
    type Slab = MaybeUninit<T::InnerType>;

    #[inline(always)]
    fn alloc() -> Self::Slab {
        MaybeUninit::uninit()
    }

    #[inline(always)]
    unsafe fn from_ptr(mem: *mut Self::Slab) -> Self {
        Slab::<T>::new(mem)
    }
}

#[macro_export]
macro_rules! ready {
    ($cs:expr, $($cond:expr),+ $(,)?) => {{
        let cs: &CriticalSection = $cs;
        $($crate::runtime::Ready::is_ready(&($cond), cs))||+
    }};
}

pub use crate::chip::Runtime;
use crate::slab::{Slab, Slabbed};
