#![no_std]
#![feature(ptr_metadata)]

extern crate alloc;

pub mod gdt;
pub mod idt;
pub mod io;
pub mod uart;
