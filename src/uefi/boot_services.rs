use core::ffi::c_void;

use super::{Status, TableHeader};

#[repr(C)]
pub struct BootServices {
    header: TableHeader,
    // Task Priority Services
    raise_tpl: *const c_void,
    restore_tpl: *const c_void,
    // Memory Services
    allocate_pages: *const c_void,
    free_pages: *const c_void,
    raw_get_memory_map: GetMemoryMap,
    allocate_pool: *const c_void,
    free_pool: *const c_void,
    // TODO: add the remaining fields
}

pub type GetMemoryMap = extern "efiapi" fn(
    size: &mut usize,
    buffer: *mut MemoryDescriptor,
    key: &mut usize,
    descriptor_size: &mut usize,
    descriptor_version: &mut u32,
) -> Status;

#[repr(C)]
pub struct MemoryDescriptor {
    ty: u32,
    physical_start: *const c_void,
    virtual_start: *const c_void,
    page_count: u64,
    attribute: u64,
}
