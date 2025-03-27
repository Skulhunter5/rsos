use core::ffi::c_void;

pub mod protocols;
pub mod tables;
pub mod services;

pub type ImageHandle = *const c_void;

#[repr(C)]
pub struct MemoryDescriptor {
    ty: u32,
    physical_start: *const c_void,
    virtual_start: *const c_void,
    page_count: u64,
    attribute: u64,
}
