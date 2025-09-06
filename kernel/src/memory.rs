use core::ops::Range;

use alloc::vec::Vec;
use common::PhysicalAddress;

pub struct PhysicalMemoryManager;

impl PhysicalMemoryManager {
    pub fn init(_memory: &Vec<Range<PhysicalAddress>>) -> Self {
        todo!();
    }
}
