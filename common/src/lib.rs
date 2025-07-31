#![no_std]
#![feature(const_trait_impl)]
#![feature(allocator_api)]
#![feature(ptr_mask)]
#![feature(pointer_is_aligned_to)]
#![feature(slice_ptr_get)]

extern crate alloc;

mod byte_size;
pub use byte_size::{BitSize, ByteSize};
pub mod allocation;
pub mod spin;
