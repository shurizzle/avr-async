#![no_std]
#![feature(asm_experimental_arch, const_mut_refs, const_trait_impl, const_pin)]

pub mod boxed;
pub mod executor;
pub mod runtime;
pub mod time;

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
            Poll::Pending
        }
    }
}

#[inline]
pub fn ayield() -> Yield {
    Yield::new()
}
