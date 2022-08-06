use avr_device::interrupt::CriticalSection;

pub trait State {
    fn snapshot(&mut self, cs: &CriticalSection);
}

pub struct Runtime<S: State> {
    pub(crate) ready: bool,
    state: S,
}

impl<S: State> Runtime<S> {
    #[inline(always)]
    pub fn new(state: S) -> Self {
        Self {
            ready: false,
            state,
        }
    }

    /// # Safety
    /// This function is marked as unsafe to remember you to call it in a critical section (usually
    /// in an interrupt)
    #[inline]
    pub unsafe fn modify<F: Fn(&mut S) -> bool>(&mut self, f: F) {
        if f(&mut self.state) {
            self.ready = true;
        }
    }

    #[inline(always)]
    pub fn is_ready(&self) -> bool {
        unsafe { core::ptr::read_volatile(&self.ready) }
    }

    #[inline]
    pub fn snapshot(&mut self, cs: &CriticalSection) {
        self.ready = false;
        self.state.snapshot(cs)
    }

    #[inline(always)]
    pub fn state(&self) -> &S {
        &self.state
    }
}
