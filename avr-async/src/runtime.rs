use avr_device::interrupt::CriticalSection;

pub trait State {
    fn snapshot(&mut self, cs: &CriticalSection);
}

pub trait Runtime {
    fn init(&mut self, cs: &CriticalSection);

    fn is_ready(&self) -> bool;

    fn snapshot(&mut self, cs: &CriticalSection);

    fn idle(&self);

    fn wake(&mut self);

    fn shutdown(&self);

    /// # Safety
    unsafe fn timer0_compa(&mut self, _cs: &CriticalSection) {}
}

pub trait StatefulRuntime<S: State>: Runtime {
    fn state(&self) -> &S;

    #[doc(hidden)]
    unsafe fn state_mut(&mut self) -> &mut S;
}
