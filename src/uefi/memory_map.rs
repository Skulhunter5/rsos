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

    pub fn iter(&self) -> MemoryMapIterator {
        MemoryMapIterator {
            inner: self,
            index: 0,
        }
    }
}

pub struct MemoryMapIterator<'a> {
    inner: &'a MemoryMap,
    index: usize,
}

impl<'a> Iterator for MemoryMapIterator<'a> {
    type Item = &'a MemoryDescriptor;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.inner.len() {
            return None;
        }
        let elem = unsafe { self.inner.data.add(self.index).as_ref()? };
        self.index += 1;
        return Some(elem);
    }
}
