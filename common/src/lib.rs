#![no_std]
#![feature(const_trait_impl)]

mod byte_size;
pub use byte_size::{BitSize, ByteSize};
pub mod spin;
