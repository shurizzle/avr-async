#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Address(u8);

impl core::fmt::Display for Address {
    #[inline(always)]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.0, f)
    }
}

#[non_exhaustive]
#[repr(transparent)]
pub struct InvalidAddress;

impl Address {
    #[inline]
    pub const fn new(me: u8) -> Result<Self, InvalidAddress> {
        if me > 0b01111111 {
            Err(InvalidAddress)
        } else {
            Ok(Self(me))
        }
    }

    #[inline]
    pub const fn const_new(me: u8) -> Self {
        if me > 0b01111111 {
            panic!("Invalid address")
        } else {
            Self(me)
        }
    }

    #[inline]
    pub const fn as_write_byte(&self) -> u8 {
        self.0 << 1
    }

    #[inline]
    pub const fn as_read_byte(&self) -> u8 {
        (self.0 << 1) | 1
    }
}

impl TryFrom<u8> for Address {
    type Error = InvalidAddress;

    #[inline(always)]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[allow(clippy::from_over_into)]
impl Into<u8> for Address {
    #[inline(always)]
    fn into(self) -> u8 {
        self.0
    }
}
