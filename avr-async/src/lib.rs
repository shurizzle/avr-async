#![no_std]
#![feature(
    asm_experimental_arch,
    abi_avr_interrupt,
    negative_impls,
    const_mut_refs,
    const_trait_impl,
    fn_traits,
    unboxed_closures
)]
#![cfg_attr(feature = "alloc", feature(allocator_api, default_alloc_error_handler))]

#[cfg(feature = "alloc")]
use core::{
    alloc::{GlobalAlloc, Layout},
    ffi::c_void,
};

#[cfg(feature = "alloc")]
extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn calloc(number: usize, size: usize) -> *mut c_void;
    fn realloc(memblock: *mut c_void, size: usize) -> *mut c_void;
}

#[cfg(feature = "alloc")]
struct GlobalAllocator;

#[cfg(feature = "alloc")]
unsafe impl GlobalAlloc for GlobalAllocator {
    #[inline(always)]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        malloc(layout.size()) as *mut u8
    }

    #[inline(always)]
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        free(ptr as *mut c_void)
    }

    #[inline(always)]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        calloc(layout.size(), 1) as *mut u8
    }

    #[inline(always)]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_layout = Layout::from_size_align_unchecked(new_size, layout.align());
        realloc(ptr as *mut c_void, new_layout.size()) as *mut u8
    }
}

#[cfg(feature = "alloc")]
#[global_allocator]
static ALLOCATOR: GlobalAllocator = GlobalAllocator;

pub(crate) mod chip;
pub mod executor;
pub mod queue;
pub mod runtime;
mod sealed;
pub mod slab;
pub(crate) mod tuple;
pub use avr_async_macros::slab;
pub mod sync;
mod sync_unsafe_cell;
pub mod task;
#[cfg(feature = "time")]
pub mod time;

pub use sync_unsafe_cell::SyncUnsafeCell;

use core::{future::Future, task::Poll};

#[derive(Default)]
pub struct Yield(bool);

impl Yield {
    #[inline]
    pub fn new() -> Self {
        Self(false)
    }
}

impl Future for Yield {
    type Output = ();

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        _cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        if self.0 {
            Poll::Ready(())
        } else {
            self.0 = true;
            unsafe { crate::executor::wake() };
            Poll::Pending
        }
    }
}

#[inline]
pub fn ayield() -> Yield {
    Yield::new()
}

#[inline]
pub fn r#yield() -> Yield {
    Yield::new()
}
