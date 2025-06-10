use core::{
    alloc::{Allocator, Layout},
    mem,
    sync::atomic::Ordering,
};

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

        if status == Status::BUFFER_TOO_SMALL {
            return Some(size);
        } else {
            return None;
        }
    }

    // TODO: get rid of the memory leak from allocating through the given allocator
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
        if status != Status::BUFFER_TOO_SMALL {
            return None;
        }

        let layout = Layout::from_size_align(size, 8).unwrap();
        let buf = alloc
            .allocate(layout)
            .expect("failed to allocate space for uefi memory map");
        let ptr = buf.as_ptr() as *mut MemoryDescriptor;

        let status = (self.table.get_memory_map)(
            &mut size,
            ptr,
            &mut key,
            &mut descriptor_size,
            &mut descriptor_version,
        );

        if status == Status::SUCCESS {
            let memory_map = unsafe {
                MemoryMap::new(
                    key,
                    ptr as *const MemoryDescriptor,
                    size / mem::size_of::<MemoryDescriptor>(),
                )
            };
            return Some(memory_map);
        }

        return None;
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
