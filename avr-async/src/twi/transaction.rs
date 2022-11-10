use super::{
    buffer::{InputBuffer, OutputBuffer},
    Address, Twi,
};

use core::{future::Future, marker::PhantomData, pin::Pin, task::Poll};

#[repr(C)]
pub enum Action<'a> {
    Write(Address, &'a mut dyn OutputBuffer),
    Read(Address, &'a mut dyn InputBuffer),
}

#[repr(C)]
pub(crate) enum RawAction {
    Write(Address, *mut dyn OutputBuffer),
    Read(Address, *mut dyn InputBuffer),
}

impl RawAction {
    pub(crate) fn from_action(a: &Action<'_>) -> *const Self {
        unsafe { ::core::mem::transmute(a) }
    }
}

#[allow(clippy::wrong_self_convention)]
pub trait IntoAction {
    fn into_action(&mut self, address: Address) -> Action;
}

pub trait Transaction<'a> {
    fn next(&mut self) -> Option<Action<'a>>;
}

pub struct SingleActionTransaction<'a> {
    action: Option<Action<'a>>,
}

impl<'a> SingleActionTransaction<'a> {
    #[inline]
    pub fn new(action: Action<'a>) -> Self {
        Self {
            action: Some(action),
        }
    }
}

impl<'a> Transaction<'a> for SingleActionTransaction<'a> {
    fn next(&mut self) -> Option<Action<'a>> {
        self.action.take()
    }
}

pub struct TransactionWait<'a, 'b, T: Transaction<'a>> {
    t: T,
    result: Option<Result<(), ()>>,
    registered: bool,
    twi: &'b mut Twi,
    _life: PhantomData<&'a ()>,
}

impl<'a, 'b, T: Transaction<'a>> !Send for TransactionWait<'a, 'b, T> {}
impl<'a, 'b, T: Transaction<'a>> !Sync for TransactionWait<'a, 'b, T> {}

impl<'a, 'b, T: Transaction<'a>> TransactionWait<'a, 'b, T> {
    pub fn new(t: T, twi: &'b mut Twi) -> Self {
        TransactionWait {
            t,
            result: None,
            registered: false,
            twi,
            _life: PhantomData,
        }
    }
}

impl<'a, 'b, T: Transaction<'a>> Future for TransactionWait<'a, 'b, T> {
    type Output = Result<(), ()>;

    fn poll(self: Pin<&mut Self>, _: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { Pin::get_unchecked_mut(self) };

        loop {
            if !this.registered {
                this.twi
                    .register(&mut this.t as &mut dyn Transaction, &mut this.result);
                this.registered = true;
            } else {
                break if this.result.is_some() {
                    Poll::Ready(unsafe { this.result.take().unwrap_unchecked() })
                } else {
                    Poll::Pending
                };
            }
        }
    }
}

impl<'a, 'b, T: Transaction<'a>> Drop for TransactionWait<'a, 'b, T> {
    fn drop(&mut self) {
        self.twi.unregister(&mut self.t as &mut dyn Transaction);
    }
}
