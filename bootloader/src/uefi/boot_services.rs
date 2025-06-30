use core::sync::atomic::Ordering;

use alloc::vec;

use super::{
    MemoryMap, Status, SystemTable,
    raw::{self, MemoryDescriptor},
};

pub struct BootServices<'a> {
    system_table: &'a SystemTable,
    table: &'a raw::tables::BootServicesTable,
}

impl<'a> BootServices<'a> {
    pub unsafe fn new(system_table: &'a SystemTable) -> Self {
        let table = unsafe { system_table.table.boot_services.as_ref().unwrap() };
        Self {
            system_table,
            table,
        }
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

        assert!(descriptor_size == size_of::<MemoryDescriptor>());

        if status == Status::BUFFER_TOO_SMALL {
            return Some(size);
        } else {
            return None;
        }
    }

    // TODO: retry getting the memory map if it doesn't succeed
    pub fn get_memory_map(&self) -> Option<MemoryMap> {
        let required_size = self.get_memory_map_size()?;
        assert!(required_size % size_of::<MemoryDescriptor>() == 0);

        let mut buffer =
            vec![MemoryDescriptor::zero(); required_size / size_of::<MemoryDescriptor>()];

        let buffer_size = buffer.len() * size_of::<MemoryDescriptor>();
        assert!(buffer_size == required_size);

        let mut size = buffer_size;
        let mut key = 0;
        let mut descriptor_size = 0;
        let mut descriptor_version = 0;

        let status = (self.table.get_memory_map)(
            &mut size,
            buffer.as_mut_ptr(),
            &mut key,
            &mut descriptor_size,
            &mut descriptor_version,
        );
        let resulting_size = size;
        assert!(resulting_size <= required_size);

        assert!(status != Status::BUFFER_TOO_SMALL);
        if status != Status::SUCCESS {
            return None;
        }

        assert!(resulting_size % size_of::<MemoryDescriptor>() == 0);
        buffer.resize(
            resulting_size / size_of::<MemoryDescriptor>(),
            MemoryDescriptor::zero(),
        );
        buffer.shrink_to_fit();
        let map = buffer.into_boxed_slice();

        Some(MemoryMap::new(key, map))
    }
}

impl Drop for BootServices<'_> {
    fn drop(&mut self) {
        self.system_table
            .boot_services_lock
            .compare_exchange(true, false, Ordering::Release, Ordering::Relaxed)
            .unwrap();
    }
}
