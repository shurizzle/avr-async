pub mod peripheral;
mod raw;
pub mod read;
pub mod transaction;
pub mod write;

use core::{future::Future, mem::MaybeUninit};

pub use address::Address;
use avr_device::interrupt::CriticalSection;
use avr_hal_generic::port;
pub use peripheral::Error;

use crate::slab::{Slab, SlabBox, Slabbed};

pub struct TwiSlab<TWI: self::peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK> {
    peripheral: self::peripheral::TwiPeripheral<TWI, SDA, SCL, CLOCK>,
    command: MaybeUninit<State>,
    set: bool,
}

impl<TWI: self::peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK> TwiSlab<TWI, SDA, SCL, CLOCK> {
    #[allow(clippy::zero_ptr)]
    #[inline(always)]
    pub fn new(peripheral: self::peripheral::TwiPeripheral<TWI, SDA, SCL, CLOCK>) -> Self {
        Self {
            peripheral,
            command: MaybeUninit::uninit(),
            set: false,
        }
    }
}

pub(crate) enum State {
    Start(Option<Result<(), Error>>),
    SlaRw(Option<Result<(), Error>>),
    Write {
        buf: *const [u8],
        idx: usize,
        res: Option<Result<(), Error>>,
    },
    Read {
        buf: *mut [u8],
        idx: usize,
        res: Option<Result<(), Error>>,
    },
    Stop(Option<()>),
}

pub struct TwoWireInterfaceDriver<TWI: self::peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK> {
    inner: SlabBox<TwiSlab<TWI, SDA, SCL, CLOCK>>,
}

impl<TWI: self::peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK>
    TwoWireInterfaceDriver<TWI, SDA, SCL, CLOCK>
{
    #[inline(always)]
    pub fn new(inner: SlabBox<TwiSlab<TWI, SDA, SCL, CLOCK>>) -> Self {
        Self { inner }
    }

    #[inline]
    pub fn run(&mut self, _: &CriticalSection) -> bool {
        if self.inner.peripheral.is_ready() && self.inner.set {
            let res = match unsafe { &mut *(self.inner.command.as_mut_ptr()) } {
                State::Start(ref mut res) => {
                    *res = Some(self.inner.peripheral.recv_start());
                    true
                }
                State::SlaRw(ref mut res) => {
                    *res = Some(self.inner.peripheral.recv_slarw());
                    true
                }
                State::Write {
                    buf,
                    ref mut idx,
                    ref mut res,
                } => match self.inner.peripheral.recv_write() {
                    Ok(()) => {
                        let buf = unsafe { &*(*buf) };
                        *idx += 1;
                        if buf.len() == *idx {
                            *res = Some(Ok(()));
                            true
                        } else {
                            self.inner.peripheral.send_write(buf[*idx]);
                            false
                        }
                    }
                    Err(err) => {
                        *res = Some(Err(err));
                        true
                    }
                },
                State::Read {
                    buf,
                    ref mut idx,
                    ref mut res,
                } => match self.inner.peripheral.recv_read() {
                    Ok(byte) => {
                        let buf = unsafe { &mut *(*buf) };
                        buf[*idx] = byte;
                        *idx += 1;

                        if buf.len() == *idx {
                            *res = Some(Ok(()));
                            true
                        } else {
                            self.inner.peripheral.send_read(buf.len() - 1 == *idx);
                            false
                        }
                    }
                    Err(err) => {
                        *res = Some(Err(err));
                        false
                    }
                },
                State::Stop(ref mut res) => {
                    *res = Some(());
                    true
                }
            };

            if res {
                self.inner.set = false;
                self.inner.peripheral.disable();
                unsafe { crate::executor::wake() };
            }

            res
        } else {
            false
        }
    }
}

impl<TWI: self::peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK> FnOnce<(&CriticalSection,)>
    for TwoWireInterfaceDriver<TWI, SDA, SCL, CLOCK>
{
    type Output = bool;

    #[inline(always)]
    extern "rust-call" fn call_once(mut self, (cs,): (&CriticalSection,)) -> Self::Output {
        self.run(cs)
    }
}

impl<TWI: self::peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK> FnMut<(&CriticalSection,)>
    for TwoWireInterfaceDriver<TWI, SDA, SCL, CLOCK>
{
    #[inline(always)]
    extern "rust-call" fn call_mut(&mut self, (cs,): (&CriticalSection,)) -> Self::Output {
        self.run(cs)
    }
}

pub struct TwoWireInterface<TWI: self::peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK> {
    inner: raw::TwoWireInterface<TWI, SDA, SCL, CLOCK>,
}

impl<TWI: self::peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK> Slabbed
    for TwoWireInterface<TWI, SDA, SCL, CLOCK>
{
    type InnerType = TwiSlab<TWI, SDA, SCL, CLOCK>;
}

impl<TWI: self::peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK>
    TwoWireInterface<TWI, SDA, SCL, CLOCK>
{
    #[inline(always)]
    pub fn new(
        inner: Slab<Self>,
        peripheral: self::peripheral::TwiPeripheral<TWI, SDA, SCL, CLOCK>,
    ) -> (Self, TwoWireInterfaceDriver<TWI, SDA, SCL, CLOCK>) {
        let (inner, driver) = raw::TwoWireInterface::new(inner, peripheral);
        (Self { inner }, driver)
    }

    #[inline]
    pub fn write<'a, 'b>(
        &'a mut self,
        addr: u8,
        buf: &'b [u8],
    ) -> write::Write<'a, 'b, TWI, SDA, SCL, CLOCK> {
        write::Write::new(&mut self.inner, addr, buf)
    }

    #[inline]
    pub fn read<'a, 'b>(
        &'a mut self,
        addr: u8,
        buf: &'b mut [u8],
    ) -> read::Read<'a, 'b, TWI, SDA, SCL, CLOCK> {
        read::Read::new(&mut self.inner, addr, buf)
    }

    #[inline(always)]
    pub fn transaction(&mut self) -> transaction::GetTransaction<TWI, SDA, SCL, CLOCK> {
        transaction::GetTransaction::new(&mut self.inner)
    }
}

pub struct Wait<'a> {
    val: &'a bool,
}

impl<'a> Wait<'a> {
    #[inline]
    pub(crate) fn new<TWI: peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK>(
        twi: &'a mut raw::TwoWireInterface<TWI, SDA, SCL, CLOCK>,
    ) -> Self {
        Self {
            val: &twi.inner.set,
        }
    }
}

impl<'a> Future for Wait<'a> {
    type Output = ();

    fn poll(
        self: core::pin::Pin<&mut Self>,
        _: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        if *self.val {
            core::task::Poll::Ready(())
        } else {
            core::task::Poll::Pending
        }
    }
}

#[cfg(feature = "atmega328p")]
pub type TwoWireInterface1<CLOCK> = TwoWireInterface<
    self::peripheral::TwiPeripheral1Pac,
    port::Pin<port::mode::Input, self::peripheral::TwiPeripheral1Sda>,
    port::Pin<port::mode::Input, self::peripheral::TwiPeripheral1Scl>,
    CLOCK,
>;

#[cfg(feature = "atmega328p")]
pub type TwoWireInterfaceDriver1<CLOCK> = TwoWireInterfaceDriver<
    self::peripheral::TwiPeripheral1Pac,
    port::Pin<port::mode::Input, self::peripheral::TwiPeripheral1Sda>,
    port::Pin<port::mode::Input, self::peripheral::TwiPeripheral1Scl>,
    CLOCK,
>;
