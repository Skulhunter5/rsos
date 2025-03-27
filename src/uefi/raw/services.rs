use crate::uefi::Status;

use super::MemoryDescriptor;

pub type GetMemoryMap = extern "efiapi" fn(
    size: &mut usize,
    buffer: *mut MemoryDescriptor,
    key: &mut usize,
    descriptor_size: &mut usize,
    descriptor_version: &mut u32,
) -> Status;

pub type Stall = extern "efiapi" fn(microseconds: usize) -> Status;
