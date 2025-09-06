#![no_std]
#![feature(const_trait_impl)]
#![feature(allocator_api)]
#![feature(ptr_mask)]
#![feature(pointer_is_aligned_to)]
#![feature(slice_ptr_get)]

#[cfg(not(target_pointer_width = "64"))]
compile_error!("unsupported target pointer width");
#[cfg(not(target_arch = "x86_64"))]
compile_error!("unsupported target pointer width");

extern crate alloc;

mod byte_size;
pub use byte_size::{BitSize, ByteSize};
mod addresses;
pub mod allocation;
mod bootinfo;
pub mod spin;
pub use addresses::{PAddr, PhysAddr, PhysicalAddress, VAddr, VirtAddr, VirtualAddress};
pub use bootinfo::BootInfo;
