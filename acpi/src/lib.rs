#![no_std]
#![feature(ptr_metadata)]

use core::ffi::c_void;

use error::AcpiError;

pub mod raw;
mod rsdp;
pub use rsdp::{Rsdp, Rsdt};
pub mod error;
mod table;
pub mod tables;
pub use table::Table;

pub type RsdpAnyVersion = c_void;

pub type OemId = [u8; 6];

pub struct AcpiTables;

impl AcpiTables {
    //pub fn from_ptr(ptr: *const RsdpAnyVersion) -> Result<AcpiTables, AcpiError> {
    //    let rsdp = unsafe { Rsdp::from_raw_ptr(ptr) }?;
    //    let rsdt = rsdp.get_rsdt()?;
    //
    //    return Ok(Self { rsdt });
    //}

    pub unsafe fn iter(ptr: *const RsdpAnyVersion) -> Result<AcpiTableIterator, AcpiError> {
        let rsdp = unsafe { Rsdp::from_raw_ptr(ptr) }?;
        let rsdt = rsdp.get_rsdt()?;

        Ok(AcpiTableIterator::new(rsdt))
    }

    pub fn from_rsdp(rsdp: Rsdp) -> Result<AcpiTableIterator, AcpiError> {
        let rsdt = rsdp.get_rsdt()?;
        Ok(AcpiTableIterator::new(rsdt))
    }
}

#[derive(Debug)]
pub struct AcpiTableIterator {
    rsdt: Rsdt,
    index: usize,
}

impl AcpiTableIterator {
    fn new(rsdt: Rsdt) -> Self {
        Self { rsdt, index: 0 }
    }
}

impl Iterator for AcpiTableIterator {
    type Item = Table;

    fn next(&mut self) -> Option<Self::Item> {
        // TODO: maybe rework this
        let item = self.rsdt.get(self.index);
        if let Some(item) = item {
            self.index += 1;
            return Some(item);
        } else {
            return None;
        }
    }
}
