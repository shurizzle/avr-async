use core::{
    borrow::{Borrow, BorrowMut},
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use avr_hal_generic::{clock::Clock, port};

pub use avr_hal_generic::i2c::{Direction, Error};

pub trait TwiOps<SDA, SCL> {
    fn setup<CLOCK: Clock>(&mut self, speed: u32);

    fn send_start(&mut self);

    fn send_slarw(&mut self, address: u8, direction: Direction);

    fn send_write(&mut self, byte: u8);

    fn send_read(&mut self, last: bool);

    fn send_stop(&mut self);

    fn is_ready(&mut self) -> bool;

    fn recv_start(&mut self) -> Result<(), Error>;

    fn recv_slarw(&mut self) -> Result<(), Error>;

    fn recv_write(&mut self) -> Result<(), Error>;

    fn recv_read(&mut self) -> Result<u8, Error>;

    fn disable(&mut self);
}

#[inline(always)]
fn sei() {
    unsafe { ::core::arch::asm!("sei") };
}

macro_rules! impl_twi {
    (:: $krate_head:ident $(:: $krate_rest:ident)*, $TWI:ident, $SDA:ident, $SCL:ident) => {
        impl_twi!(@def $TWI, $SDA, $SCL, :: $krate_head $(:: $krate_rest)*);
    };
    ($krate_head:ident $(:: $krate_rest:ident)*, $TWI:ident, $SDA:ident, $SCL:ident) => {
        impl_twi!(@def $TWI, $SDA, $SCL, $krate_head $(:: $krate_rest)*);
    };
    (@def $TWI:ident, $SDA:ident, $SCL:ident, $($krate:tt)+) => {
        impl
            $crate::twi::peripheral::TwiOps<
                $($krate)* ::port::Pin<$($krate)* ::port::mode::Input, $($krate)* ::port::$SDA>,
                $($krate)* ::port::Pin<$($krate)* ::port::mode::Input, $($krate)* ::port::$SCL>,
            > for $($krate)* ::pac::$TWI
        {
            #[inline(always)]
            fn setup<CLOCK: $crate::reexports::avr_hal_generic::clock::Clock>(&mut self, speed: u32) {
                let twbr = ((CLOCK::FREQ / speed) - 16) / 2;
                self.twbr.write(|w| unsafe { w.bits(twbr as u8) });

                // Disable prescaler
                self.twsr.write(|w| w.twps().prescaler_1());
            }

            #[inline(always)]
            fn send_start(&mut self) {
                sei();
                self.twcr.write(|w| {
                    w.twen()
                        .set_bit()
                        .twint()
                        .set_bit()
                        .twsta()
                        .set_bit()
                        .twea()
                        .set_bit()
                        .twie()
                        .set_bit()
                });
            }

            #[inline(always)]
            fn send_slarw(&mut self, address: u8, direction: $crate::twi::peripheral::Direction) {
                let dirbit = if direction == $crate::twi::peripheral::Direction::Read { 1 } else { 0 };
                let rawaddr = (address << 1) | dirbit;
                self.twdr.write(|w| unsafe { w.bits(rawaddr) });
                sei();
                // transact()
                self.twcr
                    .write(|w| w.twen().set_bit().twint().set_bit().twea().set_bit().twie().set_bit());
            }

            #[inline(always)]
            fn send_write(&mut self, byte: u8) {
                self.twdr.write(|w| unsafe { w.bits(byte) });
                sei();
                // transact()
                self.twcr
                    .write(|w| w.twen().set_bit().twint().set_bit().twie().set_bit());
            }

            #[inline(always)]
            fn send_read(&mut self, last: bool) {
                sei();
                if last {
                    self.twcr
                        .write(|w| w.twint().set_bit().twen().set_bit().twie().set_bit());
                } else {
                    self.twcr.write(|w| {
                        w.twint()
                            .set_bit()
                            .twen()
                            .set_bit()
                            .twea()
                            .set_bit()
                            .twie()
                            .set_bit()
                    });
                }
            }

            #[inline(always)]
            fn send_stop(&mut self) {
                sei();
                self.twcr.write(|w| {
                    w.twen()
                        .set_bit()
                        .twint()
                        .set_bit()
                        .twsto()
                        .set_bit()
                        .twie()
                        .set_bit()
                });
            }

            #[inline(always)]
            fn is_ready(&mut self) -> bool {
                !self.twcr.read().twint().bit_is_clear()
            }

            #[allow(unreachable_patterns)]
            #[inline(always)]
            fn recv_start(&mut self) -> Result<(), $crate::twi::peripheral::Error> {
                match self.twsr.read().tws().bits() {
                    $crate::reexports::avr_hal_generic::i2c::twi_status::TW_START | $crate::reexports::avr_hal_generic::i2c::twi_status::TW_REP_START => Ok(()),
                    $crate::reexports::avr_hal_generic::i2c::twi_status::TW_MT_ARB_LOST | $crate::reexports::avr_hal_generic::i2c::twi_status::TW_MR_ARB_LOST => {
                        Err($crate::twi::peripheral::Error::ArbitrationLost)
                    }
                    $crate::reexports::avr_hal_generic::i2c::twi_status::TW_BUS_ERROR => Err($crate::twi::peripheral::Error::BusError),
                    _ => Err($crate::twi::peripheral::Error::Unknown),
                }
            }

            #[allow(unreachable_patterns)]
            #[inline(always)]
            fn recv_slarw(&mut self) -> Result<(), $crate::twi::peripheral::Error> {
                match self.twsr.read().tws().bits() {
                    $crate::reexports::avr_hal_generic::i2c::twi_status::TW_MT_SLA_ACK | $crate::reexports::avr_hal_generic::i2c::twi_status::TW_MR_SLA_ACK => Ok(()),
                    $crate::reexports::avr_hal_generic::i2c::twi_status::TW_MT_SLA_NACK | $crate::reexports::avr_hal_generic::i2c::twi_status::TW_MR_SLA_NACK => {
                        Err($crate::twi::peripheral::Error::AddressNack)
                    }
                    $crate::reexports::avr_hal_generic::i2c::twi_status::TW_MT_ARB_LOST | $crate::reexports::avr_hal_generic::i2c::twi_status::TW_MR_ARB_LOST => {
                        Err($crate::twi::peripheral::Error::ArbitrationLost)
                    }
                    $crate::reexports::avr_hal_generic::i2c::twi_status::TW_BUS_ERROR => Err($crate::twi::peripheral::Error::BusError),
                    _ => Err($crate::twi::peripheral::Error::Unknown),
                }
            }

            #[inline(always)]
            fn recv_write(&mut self) -> Result<(), $crate::twi::peripheral::Error> {
                match self.twsr.read().tws().bits() {
                    $crate::reexports::avr_hal_generic::i2c::twi_status::TW_MT_DATA_ACK => Ok(()),
                    $crate::reexports::avr_hal_generic::i2c::twi_status::TW_MT_DATA_NACK => Err($crate::twi::peripheral::Error::DataNack),
                    $crate::reexports::avr_hal_generic::i2c::twi_status::TW_MT_ARB_LOST => Err($crate::twi::peripheral::Error::ArbitrationLost),
                    $crate::reexports::avr_hal_generic::i2c::twi_status::TW_BUS_ERROR => Err($crate::twi::peripheral::Error::BusError),
                    _ => Err($crate::twi::peripheral::Error::Unknown),
                }
            }

            #[inline(always)]
            fn recv_read(&mut self) -> Result<u8, $crate::twi::peripheral::Error> {
                match self.twsr.read().tws().bits() {
                    $crate::reexports::avr_hal_generic::i2c::twi_status::TW_MR_DATA_ACK | $crate::reexports::avr_hal_generic::i2c::twi_status::TW_MR_DATA_NACK => {
                        Ok(self.twdr.read().bits())
                    }
                    $crate::reexports::avr_hal_generic::i2c::twi_status::TW_MR_ARB_LOST => Err($crate::twi::peripheral::Error::ArbitrationLost),
                    $crate::reexports::avr_hal_generic::i2c::twi_status::TW_BUS_ERROR => Err($crate::twi::peripheral::Error::BusError),
                    _ => Err($crate::twi::peripheral::Error::Unknown),
                }
            }

            #[inline(always)]
            fn disable(&mut self) {
                self.twcr
                    .modify(|r, w| unsafe { w.bits(r.bits()).twie().clear_bit() });
            }
        }
    };
}

pub struct TwiPeripheral<TWI: TwiOps<SDA, SCL>, SDA, SCL, CLOCK> {
    #[allow(dead_code)]
    p: TWI,
    #[allow(dead_code)]
    sda: SDA,
    #[allow(dead_code)]
    scl: SCL,
    _clock: PhantomData<CLOCK>,
}

impl<TWI, SDAPIN, SCLPIN, CLOCK>
    TwiPeripheral<
        TWI,
        port::Pin<port::mode::Input, SDAPIN>,
        port::Pin<port::mode::Input, SCLPIN>,
        CLOCK,
    >
where
    TWI: TwiOps<port::Pin<port::mode::Input, SDAPIN>, port::Pin<port::mode::Input, SCLPIN>>,
    SDAPIN: port::PinOps,
    SCLPIN: port::PinOps,
    CLOCK: Clock,
{
    pub fn new(
        p: TWI,
        sda: port::Pin<port::mode::Input<port::mode::PullUp>, SDAPIN>,
        scl: port::Pin<port::mode::Input<port::mode::PullUp>, SCLPIN>,
        speed: u32,
    ) -> Self {
        let mut twi = Self {
            p,
            sda: sda.forget_imode(),
            scl: scl.forget_imode(),
            _clock: PhantomData,
        };
        twi.p.setup::<CLOCK>(speed);
        twi
    }

    pub fn with_external_pullup(
        p: TWI,
        sda: port::Pin<port::mode::Input<port::mode::Floating>, SDAPIN>,
        scl: port::Pin<port::mode::Input<port::mode::Floating>, SCLPIN>,
        speed: u32,
    ) -> Self {
        let mut twi = Self {
            p,
            sda: sda.forget_imode(),
            scl: scl.forget_imode(),
            _clock: PhantomData,
        };
        twi.p.setup::<CLOCK>(speed);
        twi
    }
}

impl<TWI: TwiOps<SDA, SCL>, SDA, SCL, CLOCK> const Deref for TwiPeripheral<TWI, SDA, SCL, CLOCK> {
    type Target = TWI;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.p
    }
}

impl<TWI: TwiOps<SDA, SCL>, SDA, SCL, CLOCK> const DerefMut
    for TwiPeripheral<TWI, SDA, SCL, CLOCK>
{
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.p
    }
}

impl<TWI: TwiOps<SDA, SCL>, SDA, SCL, CLOCK> const Borrow<TWI>
    for TwiPeripheral<TWI, SDA, SCL, CLOCK>
{
    #[inline(always)]
    fn borrow(&self) -> &TWI {
        &self.p
    }
}

impl<TWI: TwiOps<SDA, SCL>, SDA, SCL, CLOCK> const BorrowMut<TWI>
    for TwiPeripheral<TWI, SDA, SCL, CLOCK>
{
    #[inline(always)]
    fn borrow_mut(&mut self) -> &mut TWI {
        &mut self.p
    }
}

impl<TWI: TwiOps<SDA, SCL>, SDA, SCL, CLOCK> const AsRef<TWI>
    for TwiPeripheral<TWI, SDA, SCL, CLOCK>
{
    #[inline(always)]
    fn as_ref(&self) -> &TWI {
        &self.p
    }
}

impl<TWI: TwiOps<SDA, SCL>, SDA, SCL, CLOCK> const AsMut<TWI>
    for TwiPeripheral<TWI, SDA, SCL, CLOCK>
{
    #[inline(always)]
    fn as_mut(&mut self) -> &mut TWI {
        &mut self.p
    }
}

#[cfg(feature = "atmega328p")]
impl_twi!(crate::hal, TWI, PC4, PC5);

#[cfg(feature = "atmega328p")]
pub type TwiPeripheral1Pac = crate::hal::pac::TWI;

#[cfg(feature = "atmega328p")]
pub type TwiPeripheral1Sda = crate::hal::port::PC4;

#[cfg(feature = "atmega328p")]
pub type TwiPeripheral1Scl = crate::hal::port::PC5;

#[cfg(feature = "atmega328p")]
pub type TwiPeripheral1<CLOCK> = TwiPeripheral<
    TwiPeripheral1Pac,
    port::Pin<port::mode::Input, TwiPeripheral1Sda>,
    port::Pin<port::mode::Input, TwiPeripheral1Scl>,
    CLOCK,
>;

#[cfg(feature = "atmega328p")]
#[macro_export]
macro_rules! twi {
    ($peripherals:ident, $pins:ident, $clock:ty, $speed:expr) => {{
        $crate::twi::peripheral::TwiPeripheral1::<$clock>::new(
            $peripherals.TWI,
            $pins.pc4.into_pull_up_input(),
            $pins.pc5.into_pull_up_input(),
            $speed,
        )
    }};
}

#[cfg(feature = "atmega328p")]
#[macro_export]
macro_rules! twi_external_pullup {
    ($peripherals:ident, $pins:ident, $clock:ty, $speed:expr) => {{
        $crate::twi::peripheral::TwiPeripheral1::<$clock>::with_external_pullup(
            $peripherals.TWI,
            $pins.pc4.into_floating_input(),
            $pins.pc5.into_floating_input(),
            $speed,
        )
    }};
}
