use core::{ffi::c_void, marker::PhantomData};

use super::Guid;

#[repr(C)]
pub struct ConfigurationTableEntry {
    pub guid: Guid,
    pub ptr: *mut c_void,
}

pub struct ConfigurationTable<'a> {
    count: usize,
    data: *const ConfigurationTableEntry,
    marker: PhantomData<&'a ConfigurationTableEntry>,
}

impl ConfigurationTable<'_> {
    pub fn new(count: usize, data: *const ConfigurationTableEntry) -> Self {
        Self {
            count,
            data,
            marker: PhantomData,
        }
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn iter(&self) -> ConfigurationTableIterator {
        ConfigurationTableIterator {
            table: self,
            index: 0,
        }
    }
}

pub struct ConfigurationTableIterator<'a> {
    table: &'a ConfigurationTable<'a>,
    index: usize,
}

impl<'a> Iterator for ConfigurationTableIterator<'a> {
    type Item = &'a ConfigurationTableEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.table.len() {
            return None;
        }

        let item = unsafe {
            self.table
                .data
                .add(self.index)
                .as_ref()
                .expect("invalid UEFI configuration table")
        };
        self.index += 1;
        return Some(item);
    }
}
