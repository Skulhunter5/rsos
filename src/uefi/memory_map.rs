use super::raw::MemoryDescriptor;

#[derive(Debug)]
pub struct MemoryMap {
    pub key: usize,
    data: *const MemoryDescriptor,
    count: usize,
}

impl MemoryMap {
    #[inline]
    pub unsafe fn new(key: usize, data: *const MemoryDescriptor, count: usize) -> Self {
        Self { key, data, count }
    }

    #[inline]
    pub fn len(&self) -> usize {
        return self.count;
    }

    pub fn get(&self, index: usize) -> Option<&MemoryDescriptor> {
        if index >= self.count {
            return None;
        }
        return unsafe { self.data.add(index).as_ref() };
    }
}
