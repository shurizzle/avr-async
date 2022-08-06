use arduino_hal::pac::CPU;
use avr_device::interrupt::CriticalSection;

pub trait State {
    fn snapshot(&mut self, cs: &CriticalSection);
}

pub trait Runtime<S: State> {
    /// # Safety
    /// This function is marked as unsafe to remember you to call it in a critical section (usually
    /// in an interrupt)
    unsafe fn modify<F: Fn(&mut S) -> bool>(&mut self, f: F);

    fn is_ready(&self) -> bool;

    fn snapshot(&mut self, cs: &CriticalSection);

    fn state(&self) -> &S;

    fn idle(&self);

    fn shutdown(&self);
}

pub struct DefaultRuntime<S: State + 'static> {
    pub(crate) ready: bool,
    state: S,
    cpu: CPU,
}

impl<S: State> DefaultRuntime<S> {
    #[inline(always)]
    pub fn new(state: S, cpu: CPU) -> Self {
        Self {
            ready: false,
            state,
            cpu,
        }
    }
}

impl<S: State> Runtime<S> for DefaultRuntime<S> {
    unsafe fn modify<F: Fn(&mut S) -> bool>(&mut self, f: F) {
        if f(&mut self.state) {
            self.ready = true;
        }
    }

    #[inline(always)]
    fn is_ready(&self) -> bool {
        unsafe { core::ptr::read_volatile(&self.ready) }
    }

    #[inline]
    fn snapshot(&mut self, cs: &CriticalSection) {
        self.ready = false;
        self.state.snapshot(cs)
    }

    #[inline(always)]
    fn state(&self) -> &S {
        &self.state
    }

    #[inline]
    fn idle(&self) {
        self.cpu.smcr.write(|w| w.sm().idle().se().set_bit());
        unsafe { ::core::arch::asm!("sleep") };
    }

    #[inline]
    fn shutdown(&self) {
        self.cpu.smcr.write(|w| w.sm().pdown().se().set_bit());
        unsafe { ::core::arch::asm!("sleep") };
    }
}
