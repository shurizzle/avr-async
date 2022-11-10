mod address;
mod buffer;
pub mod driver;
pub mod peripheral;
mod transaction;

pub use address::Address;
use driver::Driver;
pub use transaction::*;

pub struct Twi;

impl Twi {
    pub fn register(
        &mut self,
        _transaction: &mut dyn Transaction,
        _result: &mut Option<Result<(), ()>>,
    ) {
        unimplemented!()
    }

    pub fn unregister(&mut self, _transaction: &mut dyn Transaction) {
        unimplemented!()
    }
}
