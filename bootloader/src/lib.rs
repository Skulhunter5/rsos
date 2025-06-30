#![no_std]
#![feature(ptr_metadata)]

extern crate alloc;

pub mod acpi;
pub mod cursor;
pub mod elf;
pub mod io;
pub mod pci;
pub mod spin;
pub mod allocator;
