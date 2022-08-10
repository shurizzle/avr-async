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

pub use crate::chip::Runtime;
