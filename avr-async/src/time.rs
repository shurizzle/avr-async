use core::{future::Future, marker::PhantomData, task::Poll};

use avr_device::interrupt::{self, CriticalSection};
use num_traits::{Bounded, CheckedAdd, NumAssignOps, One, Unsigned, Zero};

use crate::runtime::State;

pub trait UInt: Unsigned + Copy + NumAssignOps + Ord + Bounded + CheckedAdd {}

impl<I: Unsigned + Copy + NumAssignOps + Ord + Bounded + CheckedAdd> UInt for I {}

pub struct TickCounter<I: UInt> {
    counter: I,
    snapshot: I,
}

impl<I: UInt> Default for TickCounter<I> {
    #[inline]
    fn default() -> Self {
        Self {
            counter: Zero::zero(),
            snapshot: Zero::zero(),
        }
    }
}

impl<I: UInt> TickCounter<I> {
    #[inline(always)]
    pub fn new() -> Self {
        Default::default()
    }

    /// # Safety
    /// This function is marked as unsafe to remember you to call it in a critical section (usually
    /// an interrupt)
    #[inline(always)]
    pub unsafe fn inc(&mut self) {
        self.counter += One::one();
    }

    #[inline(always)]
    pub fn get(&self) -> I {
        self.snapshot
    }

    #[inline]
    pub fn get_real(&self) -> I {
        interrupt::free(|_cs| self.counter)
    }

    #[inline(always)]
    pub fn delay<'a, 'b: 'a>(&'b self, delay: I) -> TickDelay<'a, I> {
        TickDelay::new(self, delay)
    }

    #[inline(always)]
    pub fn interval<'a, 'b: 'a>(&'b self, interval: I) -> TickInterval<'a, I> {
        TickInterval::new(self, interval)
    }
}

impl<I: UInt> State for TickCounter<I> {
    #[inline(always)]
    fn snapshot(&mut self, _cs: &CriticalSection) {
        self.snapshot = self.counter;
    }
}

#[inline(always)]
fn elapsed<I: UInt>(start: I, now: I) -> Option<I> {
    if start > now {
        (I::max_value() - start).checked_add(&now)
    } else {
        Some(now - start)
    }
}

pub struct TickDelay<'a, I: UInt> {
    start: I,
    counter: *const TickCounter<I>,
    delay: I,
    _life: PhantomData<&'a ()>,
}

impl<'a, I: UInt> TickDelay<'a, I> {
    #[inline(always)]
    pub fn new<'b: 'a>(tick: &'b TickCounter<I>, delay: I) -> Self {
        TickDelay {
            start: tick.get(),
            counter: tick as *const _,
            delay,
            _life: PhantomData,
        }
    }
}

impl<'a, I: UInt> Future for TickDelay<'a, I> {
    type Output = ();

    fn poll(
        self: core::pin::Pin<&mut Self>,
        _cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        if elapsed(self.start, unsafe { &*self.counter }.get())
            .map(|x| x >= self.delay)
            .unwrap_or(true)
        {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

pub struct TickInterval<'a, I: UInt> {
    counter: *const TickCounter<I>,
    interval: I,
    _life: PhantomData<&'a ()>,
}

impl<'a, I: UInt> TickInterval<'a, I> {
    #[inline(always)]
    pub fn new<'b: 'a>(tick: &'b TickCounter<I>, interval: I) -> Self {
        TickInterval {
            interval,
            counter: tick as *const _,
            _life: PhantomData,
        }
    }

    #[inline(always)]
    pub fn next(&'a self) -> TickDelay<'a, I> {
        TickDelay::new(unsafe { &*self.counter }, self.interval)
    }
}
