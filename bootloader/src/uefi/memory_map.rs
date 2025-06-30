// use core::ops::{Index, Range, RangeInclusive, RangeTo};
use core::ops::Index;

use alloc::boxed::Box;

use super::raw::MemoryDescriptor;

#[derive(Debug)]
pub struct MemoryMap {
    key: usize,
    data: Box<[MemoryDescriptor]>,
}

impl MemoryMap {
    pub fn new(key: usize, data: Box<[MemoryDescriptor]>) -> Self {
        Self { key, data }
    }

    pub fn get_key(&self) -> usize {
        self.key
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn iter(&self) -> core::slice::Iter<'_, MemoryDescriptor> {
        self.data.iter()
    }
}

impl Index<usize> for MemoryMap {
    type Output = MemoryDescriptor;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

macro_rules! range_index_impl {
    ( $($name:ident),* $(,)? ) => {
        $(
            use core::ops::$name;
            impl Index<$name<usize>> for MemoryMap {
                type Output = [MemoryDescriptor];

                fn index(&self, index: $name<usize>) -> &Self::Output {
                    &self.data[index]
                }
            }
        )*
    }
}

range_index_impl!(Range, RangeInclusive, RangeTo, RangeToInclusive, RangeFrom);

use core::ops::RangeFull;
impl Index<RangeFull> for MemoryMap {
    type Output = [MemoryDescriptor];

    fn index(&self, index: RangeFull) -> &Self::Output {
        &self.data[index]
    }
}
