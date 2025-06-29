use alloc::boxed::Box;

use super::raw::MemoryDescriptor;

#[derive(Debug)]
pub struct MemoryMap {
    pub key: usize,
    pub data: Box<[MemoryDescriptor]>,
}

impl MemoryMap {
    pub fn new(key: usize, data: Box<[MemoryDescriptor]>) -> Self {
        Self { key, data }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn iter(&self) -> core::slice::Iter<'_, MemoryDescriptor> {
        self.data.iter()
    }
}
