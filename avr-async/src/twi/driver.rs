pub struct Driver<TWI: super::peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK> {
    state: Option<TransactionState>,
    set: bool,
    peripheral: super::peripheral::TwiPeripheral<TWI, SDA, SCL, CLOCK>,
}

impl<TWI: super::peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK> Driver<TWI, SDA, SCL, CLOCK> {
    #[allow(clippy::new_without_default)]
    #[inline(always)]
    pub(crate) fn new(peripheral: super::peripheral::TwiPeripheral<TWI, SDA, SCL, CLOCK>) -> Self {
        Self {
            state: None,
            set: false,
            peripheral,
        }
    }
}

pub struct TransactionState;
