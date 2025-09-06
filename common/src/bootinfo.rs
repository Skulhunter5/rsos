use alloc::vec::Vec;

use crate::PhysicalAddress;

pub struct BootInfo {
    pub available_mem: Vec<core::ops::Range<PhysicalAddress>>,
}
