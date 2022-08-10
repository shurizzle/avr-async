use avr_device::interrupt::CriticalSection;

pub trait Ready {
    fn is_ready(&self, cs: &CriticalSection) -> bool;
}

#[macro_export]
macro_rules! ready {
    ($cs:expr, $($cond:expr),+ $(,)?) => {{
        let cs: &CriticalSection = $cs;
        $($crate::runtime::Ready::is_ready(&($cond), cs))||+
    }};
}

pub trait Runtime: Ready {
    fn init(&mut self, cs: &CriticalSection);

    fn snapshot(&mut self, cs: &CriticalSection);

    fn idle(&self);

    fn wake(&mut self);

    fn shutdown(&self);

    /// # Safety
    unsafe fn timer0_compa(&mut self, _cs: &CriticalSection) {}
}
