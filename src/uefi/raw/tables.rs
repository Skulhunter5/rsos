use core::{ffi::c_void, sync::atomic::AtomicPtr};

use crate::uefi::Guid;

use super::{protocols::SimpleTextOutputProtocol, services::{GetMemoryMap, Stall}, ImageHandle};

#[repr(C)]
pub struct TableHeader {
    pub signature: u64,
    pub revision: u32,
    pub size: u32,
    pub crc32: u32,
    pub reserved: u32,
}

#[repr(C)]
pub struct SystemTable {
    pub header: TableHeader,
    pub firmware_vendor: *const c_void,
    pub firmware_revision: u32,
    pub console_in_handle: ImageHandle,
    pub con_in: *const c_void,
    pub console_out_handle: ImageHandle,
    pub con_out: AtomicPtr<SimpleTextOutputProtocol>,
    pub standard_error_handle: ImageHandle,
    pub std_err: AtomicPtr<SimpleTextOutputProtocol>,
    pub runtime_services: *const c_void,
    pub boot_services: *mut BootServicesTable,
    pub number_of_table_entries: usize,
    pub configuration_table: *const ConfigurationTableEntry,
}

#[repr(C)]
pub struct ConfigurationTableEntry {
    pub guid: Guid,
    pub ptr: *mut c_void,
}

#[repr(C)]
pub struct BootServicesTable {
    pub header: TableHeader,
    // Task Priority Services
    pub raise_tpl: *const c_void,
    pub restore_tpl: *const c_void,
    // Memory Services
    pub allocate_pages: *const c_void,
    pub free_pages: *const c_void,
    pub get_memory_map: GetMemoryMap,
    pub allocate_pool: *const c_void,
    pub free_pool: *const c_void,
    // Event & Timer Services
    pad0: [*const c_void; 6],
    // Protocol Handler Services
    pad1: [*const c_void; 9],
    // Image Services
    pad2: [*const c_void; 5],
    // Miscellaneous Services
    pub get_next_monotonic_count: *const c_void,
    pub stall: Stall,
    pub set_watchdog_timer: *const c_void,
    // TODO: add the remaining fields and version support
}
