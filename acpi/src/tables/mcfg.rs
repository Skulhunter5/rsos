use core::ptr;

use crate::raw::{self, ConfigurationSpaceBaseAddressAllocation};

#[derive(Debug)]
pub struct Mcfg(pub(crate) *const raw::Mcfg);

impl Mcfg {
    pub fn len(&self) -> usize {
        ptr::metadata(self.0)
    }

    pub fn get(&self, _index: usize) -> ConfigurationSpaceBaseAddressAllocation {
        todo!();
    }
}
