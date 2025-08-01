use core::ptr;

use crate::{
    raw::{self, ConfigurationSpaceBaseAddressAllocation, SdtHeader},
    tables::Mcfg,
};

#[derive(Debug)]
pub enum Table {
    Mcfg(Mcfg),
    Other(*const SdtHeader),
}

impl Table {
    pub unsafe fn from(ptr: *const SdtHeader) -> Self {
        let header = unsafe { &*ptr };
        let size = header.length as usize - size_of::<SdtHeader>();
        match header.signature {
            raw::MCFG_SIGNATURE => {
                // Remove the size for the additional reserved field before the MCFG array
                let size = size - 8;
                let count = size / size_of::<ConfigurationSpaceBaseAddressAllocation>();
                let ptr: *const raw::Mcfg = ptr::from_raw_parts(ptr as *const (), count);
                Self::Mcfg(Mcfg(ptr))
            }
            _ => Self::Other(ptr),
        }
    }
}
