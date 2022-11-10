use core::{future::Future, marker::PhantomData, task::Poll};

use crate::slab::{Slab, SlabBox, Slabbed};

pub(crate) struct TwoWireInterface<TWI: super::peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK> {
    pub inner: SlabBox<super::TwiSlab<TWI, SDA, SCL, CLOCK>>,
}

impl<TWI: super::peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK> Slabbed
    for TwoWireInterface<TWI, SDA, SCL, CLOCK>
{
    type InnerType = super::TwiSlab<TWI, SDA, SCL, CLOCK>;
}

impl<TWI: super::peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK>
    TwoWireInterface<TWI, SDA, SCL, CLOCK>
{
    #[inline(always)]
    pub fn new(
        inner: Slab<super::TwoWireInterface<TWI, SDA, SCL, CLOCK>>,
        peripheral: super::peripheral::TwiPeripheral<TWI, SDA, SCL, CLOCK>,
    ) -> (Self, super::TwoWireInterfaceDriver<TWI, SDA, SCL, CLOCK>) {
        let inner = inner.get(super::TwiSlab::new(peripheral));
        let inner2 = unsafe {
            SlabBox::from_ptr(SlabBox::as_ptr(&inner) as *mut super::TwiSlab<TWI, SDA, SCL, CLOCK>)
        };

        (Self { inner }, super::TwoWireInterfaceDriver::new(inner2))
    }

    #[inline]
    pub fn start(&mut self) -> Start {
        Start::new(self.inner.as_mut())
    }

    #[inline]
    pub fn sla_rw(&mut self, addr: u8, direction: super::peripheral::Direction) -> SlaRw {
        SlaRw::new(self.inner.as_mut(), addr, direction)
    }

    #[inline]
    pub fn write<'a, 'b>(&'a mut self, buf: &'b [u8]) -> Write<'a, 'b> {
        Write::new(self.inner.as_mut(), buf)
    }

    #[inline]
    pub fn read<'a, 'b>(&'a mut self, buf: &'b mut [u8]) -> Read<'a, 'b> {
        Read::new(self.inner.as_mut(), buf)
    }

    #[inline]
    pub fn stop(&mut self) -> Stop {
        Stop::new(self.inner.as_mut())
    }

    #[inline]
    pub(crate) fn stop_unbound(&mut self) {
        self.inner.command.write(super::State::Stop(None));
        self.inner.set = true;
        self.inner.peripheral.send_stop();
    }
}

pub struct Start<'a> {
    state: &'a mut super::State,
}

impl<'a> Start<'a> {
    pub fn new<TWI: super::peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK>(
        inner: &'a mut super::TwiSlab<TWI, SDA, SCL, CLOCK>,
    ) -> Self {
        inner.command.write(super::State::Start(None));
        inner.set = true;
        inner.peripheral.send_start();
        Self {
            state: unsafe { &mut *(inner.command.as_mut_ptr() as *mut super::State) },
        }
    }
}

impl<'a> Future for Start<'a> {
    type Output = Result<(), super::Error>;

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        _: &mut core::task::Context<'_>,
    ) -> Poll<Self::Output> {
        if let super::State::Start(ref mut res) = self.state {
            if let Some(res) = res.take() {
                Poll::Ready(res)
            } else {
                Poll::Pending
            }
        } else {
            panic!("Wrong TWI state")
        }
    }
}

pub struct SlaRw<'a> {
    state: &'a mut super::State,
}

impl<'a> SlaRw<'a> {
    pub fn new<TWI: super::peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK>(
        inner: &'a mut super::TwiSlab<TWI, SDA, SCL, CLOCK>,
        addr: u8,
        direction: super::peripheral::Direction,
    ) -> Self {
        inner.command.write(super::State::SlaRw(None));
        inner.set = true;
        inner.peripheral.send_slarw(addr, direction);
        Self {
            state: unsafe { &mut *(inner.command.as_mut_ptr() as *mut super::State) },
        }
    }
}

impl<'a> Future for SlaRw<'a> {
    type Output = Result<(), super::Error>;

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        _: &mut core::task::Context<'_>,
    ) -> Poll<Self::Output> {
        if let super::State::SlaRw(ref mut res) = self.state {
            if let Some(res) = res.take() {
                Poll::Ready(res)
            } else {
                Poll::Pending
            }
        } else {
            panic!("Wrong TWI state")
        }
    }
}

pub struct Write<'a, 'b> {
    state: Option<&'a mut super::State>,
    _life: PhantomData<&'b ()>,
}

impl<'a, 'b> Write<'a, 'b> {
    pub fn new<TWI: super::peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK>(
        inner: &'a mut super::TwiSlab<TWI, SDA, SCL, CLOCK>,
        buf: &'b [u8],
    ) -> Self {
        if buf.is_empty() {
            Self {
                state: None,
                _life: PhantomData,
            }
        } else {
            inner.command.write(super::State::Write {
                buf: buf as *const [u8],
                idx: 0,
                res: None,
            });
            inner.set = true;
            inner.peripheral.send_write(buf[0]);
            Self {
                state: Some(unsafe { &mut *(inner.command.as_mut_ptr() as *mut super::State) }),
                _life: PhantomData,
            }
        }
    }
}

impl<'a, 'b> Future for Write<'a, 'b> {
    type Output = Result<(), super::Error>;

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        _: &mut core::task::Context<'_>,
    ) -> Poll<Self::Output> {
        if let Some(ref mut state) = self.state {
            if let super::State::Write { ref mut res, .. } = state {
                if let Some(res) = res.take() {
                    Poll::Ready(res)
                } else {
                    Poll::Pending
                }
            } else {
                panic!("Wrong TWI state")
            }
        } else {
            Poll::Ready(Ok(()))
        }
    }
}

pub struct Read<'a, 'b> {
    state: Option<&'a mut super::State>,
    _buf: &'b mut [u8],
}

impl<'a, 'b> Read<'a, 'b> {
    pub fn new<TWI: super::peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK>(
        inner: &'a mut super::TwiSlab<TWI, SDA, SCL, CLOCK>,
        buf: &'b mut [u8],
    ) -> Self {
        if buf.is_empty() {
            Self {
                state: None,
                _buf: buf,
            }
        } else {
            inner.command.write(super::State::Read {
                buf: buf as *mut [u8],
                idx: 0,
                res: None,
            });
            inner.set = true;
            inner.peripheral.send_read(buf.len() == 1);
            Self {
                state: Some(unsafe { &mut *(inner.command.as_mut_ptr() as *mut super::State) }),
                _buf: buf,
            }
        }
    }
}

impl<'a, 'b> Future for Read<'a, 'b> {
    type Output = Result<(), super::Error>;

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        _: &mut core::task::Context<'_>,
    ) -> Poll<Self::Output> {
        if let Some(ref mut state) = self.state {
            if let super::State::Read { ref mut res, .. } = state {
                if let Some(res) = res.take() {
                    Poll::Ready(res)
                } else {
                    Poll::Pending
                }
            } else {
                panic!("Wrong TWI state")
            }
        } else {
            Poll::Ready(Ok(()))
        }
    }
}

pub struct Stop<'a> {
    state: &'a mut super::State,
}

impl<'a> Stop<'a> {
    pub fn new<TWI: super::peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK>(
        inner: &'a mut super::TwiSlab<TWI, SDA, SCL, CLOCK>,
    ) -> Self {
        inner.command.write(super::State::Stop(None));
        inner.set = true;
        inner.peripheral.send_stop();
        Self {
            state: unsafe { &mut *(inner.command.as_mut_ptr() as *mut super::State) },
        }
    }
}

impl<'a> Future for Stop<'a> {
    type Output = ();

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        _: &mut core::task::Context<'_>,
    ) -> Poll<Self::Output> {
        if let super::State::Stop(ref mut res) = self.state {
            if let Some(res) = res.take() {
                Poll::Ready(res)
            } else {
                Poll::Pending
            }
        } else {
            panic!("Wrong TWI state")
        }
    }
}
