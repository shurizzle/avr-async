use core::{future::Future, task::Poll};

pub mod read;
pub mod write;

pub struct GetTransaction<'a, TWI: super::peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK> {
    twi: Option<&'a mut super::raw::TwoWireInterface<TWI, SDA, SCL, CLOCK>>,
}

impl<'a, TWI: super::peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK>
    GetTransaction<'a, TWI, SDA, SCL, CLOCK>
{
    #[inline]
    pub(crate) fn new(twi: &'a mut super::raw::TwoWireInterface<TWI, SDA, SCL, CLOCK>) -> Self {
        Self { twi: Some(twi) }
    }
}

impl<'a, TWI: super::peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK> Future
    for GetTransaction<'a, TWI, SDA, SCL, CLOCK>
{
    type Output = Transaction<'a, TWI, SDA, SCL, CLOCK>;

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        _: &mut core::task::Context<'_>,
    ) -> Poll<Self::Output> {
        let twi = self.twi.take().unwrap();
        if twi.inner.set {
            Poll::Ready(Transaction::new(twi))
        } else {
            self.twi.replace(twi);
            Poll::Pending
        }
    }
}

pub struct Transaction<'a, TWI: super::peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK> {
    twi: &'a mut super::raw::TwoWireInterface<TWI, SDA, SCL, CLOCK>,
    started: bool,
}

impl<'a, TWI: super::peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK>
    Transaction<'a, TWI, SDA, SCL, CLOCK>
{
    #[inline]
    fn new(twi: &'a mut super::raw::TwoWireInterface<TWI, SDA, SCL, CLOCK>) -> Self {
        Self {
            twi,
            started: false,
        }
    }

    pub fn write<'b, 'c>(
        &'b mut self,
        addr: u8,
        buf: &'c [u8],
    ) -> write::Write<'a, 'b, 'c, TWI, SDA, SCL, CLOCK> {
        write::Write::new(self, addr, buf)
    }

    pub fn read<'b, 'c>(
        &'b mut self,
        addr: u8,
        buf: &'c mut [u8],
    ) -> read::Read<'a, 'b, 'c, TWI, SDA, SCL, CLOCK> {
        read::Read::new(self, addr, buf)
    }
}
impl<'a, TWI: super::peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK> Drop
    for Transaction<'a, TWI, SDA, SCL, CLOCK>
{
    fn drop(&mut self) {
        if self.started {
            self.twi.stop_unbound()
        }
    }
}
