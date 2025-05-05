use core::{ffi::c_void, mem};

use raw::{ConfigurationSpaceBaseAddressAllocation, SdtHeader};

pub mod raw;

pub type RsdpAnyVersion = c_void;

pub type OemId = [u8; 6];

#[derive(Debug, Clone, Copy)]
pub enum AcpiError {
    InvalidChecksum,
    RsdpError(RsdpError),
}

#[derive(Debug, Clone, Copy)]
pub enum RsdpError {
    NullPointer,
    InvalidRevision(u8),
    InvalidChecksum,
    InvalidSignature([u8; 8]),
}

impl From<RsdpError> for AcpiError {
    fn from(e: RsdpError) -> Self {
        Self::RsdpError(e)
    }
}

#[derive(Debug)]
pub enum Rsdp {
    Rsdp(*const raw::Rsdp),
    Xsdp(*const raw::Xsdp),
}

impl Rsdp {
    pub unsafe fn from_raw_ptr(ptr: *const RsdpAnyVersion) -> Result<Self, RsdpError> {
        let rsdp_ptr = ptr as *const raw::Rsdp;
        let rsdp = match unsafe { rsdp_ptr.as_ref() } {
            Some(ptr) => ptr,
            None => return Err(RsdpError::NullPointer),
        };

        if rsdp.signature != raw::RSDP_SIGNATURE {
            return Err(RsdpError::InvalidSignature(rsdp.signature));
        }

        let revision = rsdp.revision;

        match revision {
            0 => {
                if unsafe { !rsdp.validate() } {
                    return Err(RsdpError::InvalidChecksum);
                }
                return Ok(Rsdp::Rsdp(rsdp_ptr));
            }
            2 => {
                let xsdp_ptr = rsdp_ptr as *const raw::Xsdp;
                let xsdp = unsafe { xsdp_ptr.as_ref().expect("unreachable, pointer already checked at the start") };
                if unsafe { !xsdp.validate() } {
                    return Err(RsdpError::InvalidChecksum);
                }
                return Ok(Rsdp::Xsdp(xsdp_ptr));
            }
            _ => return Err(RsdpError::InvalidRevision(rsdp.revision)),
        }
    }

    pub fn revision(&self) -> u8 {
        match self {
            Self::Rsdp(rsdp) => unsafe { &**rsdp }.revision,
            Self::Xsdp(xsdp) => unsafe { &**xsdp }.rsdp.revision,
        }
    }

    pub fn oem_id(&self) -> &OemId {
        match self {
            Self::Rsdp(rsdp) => &unsafe { &**rsdp }.oem_id,
            Self::Xsdp(xsdp) => &unsafe { &**xsdp }.rsdp.oem_id,
        }
    }

    // TODO: find a way to prevent truncating the xsdt_pointer on systems in 32 bit mode
    pub fn get_rsdt(&self) -> Result<Rsdt, AcpiError> {
        match self {
            Self::Rsdp(rsdp) => {
                let rsdp = unsafe { &**rsdp };
                if unsafe { !rsdp.validate() } {
                    return Err(AcpiError::InvalidChecksum);
                }
                let rsdt = unsafe { rsdp.rsdt() };

                Ok(Rsdt::Rsdt(rsdt))
            }
            Self::Xsdp(xsdp) => {
                let xsdp = unsafe { &**xsdp };
                if unsafe { !xsdp.validate() } {
                    return Err(AcpiError::InvalidChecksum);
                }
                let xsdt = unsafe { xsdp.xsdt() };

                Ok(Rsdt::Xsdt(xsdt))
            }
        }
    }
}

#[derive(Debug)]
pub enum Rsdt {
    Rsdt(*const raw::Rsdt),
    Xsdt(*const raw::Xsdt),
}

impl Rsdt {
    pub fn get(&self, index: usize) -> Option<Table> {
        match self {
            Self::Rsdt(rsdt) => {
                let rsdt = unsafe { &**rsdt };
                if index >= rsdt.tables.len() {
                    return None;
                }
                let address: u32 = rsdt.tables[index];
                let header_ptr = address as *const SdtHeader;
                let table = unsafe { Table::from(header_ptr) };
                Some(table)
            }
            Self::Xsdt(xsdt) => {
                let xsdt = unsafe { &**xsdt };
                if index >= xsdt.tables.len() {
                    return None;
                }
                crate::println!("tables: {:?}", &xsdt.tables);
                let address: u64 = xsdt.tables[index];
                crate::println!("ptr: {:x}", address);
                let header_ptr = address as *const SdtHeader;
                let table = unsafe { Table::from(header_ptr) };
                Some(table)
            }
        }
    }

    pub unsafe fn get_mcfg() -> Mcfg {
        todo!();
    }
}

#[derive(Debug)]
pub enum Table {
    Mcfg(Mcfg),
    Other(*const SdtHeader),
}

impl Table {
    unsafe fn from(ptr: *const SdtHeader) -> Self {
        crate::println!("ptr: {:?}", ptr);
        let header = unsafe { &*ptr };
        let size = header.length as usize - mem::size_of::<SdtHeader>();
        crate::println!("sig: {:?}", header.signature);
        match header.signature {
            raw::MCFG_SIGNATURE => {
                // Remove the size for the additional reserved field before the MCFG array
                let size = size - 8;
                let count = size / mem::size_of::<ConfigurationSpaceBaseAddressAllocation>();
                let ptr: *const raw::Mcfg = core::ptr::from_raw_parts(ptr as *const (), count);
                Self::Mcfg(Mcfg(ptr))
            },
            _ => Self::Other(ptr),
        }
    }
}

#[derive(Debug)]
pub struct Mcfg(*const raw::Mcfg);

impl Mcfg {
    pub fn len(&self) -> usize {
        todo!();
    }
}

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
