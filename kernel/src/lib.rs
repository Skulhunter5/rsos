#![no_std]
#![feature(new_zeroed_alloc)]
#![feature(ptr_metadata)]

extern crate alloc;

pub mod gdt;
pub mod idt;
pub mod io;
pub mod uart;
