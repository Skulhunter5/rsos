use core::{alloc::Allocator, sync::atomic::Ordering};

use super::{raw, Status, SystemTable};

pub struct MemoryMap {
    pub key: usize,
}

impl MemoryMap {
    #[inline]
    pub fn new(key: usize) -> Self {
        Self { key }
    }
}

pub struct BootServices<'a> {
    system_table: &'a SystemTable,
    table: &'a raw::tables::BootServicesTable,
}

impl<'a> BootServices<'a> {
    pub fn new(system_table: &'a SystemTable) -> Self {
        let table = unsafe { system_table.table.boot_services.as_ref().unwrap() };
        Self { system_table, table }
    }
}

impl BootServices<'_> {
    #[inline(always)]
    pub fn stall_us(&self, microseconds: usize) {
        // SAFETY: EFI_BOOT_SERVICES.Stall() can't fail according to specification
        let _status = (self.table.stall)(microseconds);
    }

    pub fn get_memory_map_size(&self) -> Option<usize> {
        let mut size = 0;
        let mut key = 0;
        let mut descriptor_size = 0;
        let mut descriptor_version = 0;

        let status = (self.table.get_memory_map)(
            &mut size,
            core::ptr::null_mut(),
            &mut key,
            &mut descriptor_size,
            &mut descriptor_version,
        );

        if status == Status::BUFFER_TOO_SMALL {
            return Some(size);
        } else {
            return None;
        }
    }

    pub fn get_memory_map<A: Allocator>(&self, alloc: A) -> Option<MemoryMap> {
        let mut size = 0;
        let mut key = 0;
        let mut descriptor_size = 0;
        let mut descriptor_version = 0;
        let status = (self.table.get_memory_map)(
            &mut size,
            core::ptr::null_mut(),
            &mut key,
            &mut descriptor_size,
            &mut descriptor_version,
        );

        todo!();

        Some(MemoryMap::new(size))
    }
}

impl Drop for BootServices<'_> {
    fn drop(&mut self) {
        self.system_table.boot_services_lock.compare_exchange(true, false, Ordering::Release, Ordering::Relaxed).unwrap();
    }
}
