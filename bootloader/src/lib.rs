#![no_std]
#![feature(ptr_metadata)]
#![feature(allocator_api)]
#![feature(slice_ptr_get)]

extern crate alloc;

pub mod acpi;
pub mod allocator;
pub mod cursor;
pub mod elf;
pub mod io;
pub mod pci;
pub mod uefi2;
