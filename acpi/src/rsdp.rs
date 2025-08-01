use crate::{
    OemId, RsdpAnyVersion, Table,
    error::{AcpiError, RsdpError},
    raw::{self, SdtHeader},
    tables::Mcfg,
};

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
                let xsdp = unsafe {
                    xsdp_ptr
                        .as_ref()
                        .expect("unreachable, pointer already checked at the start")
                };
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
                // TODO: validate rsdt

                Ok(Rsdt::Rsdt(rsdt))
            }
            Self::Xsdp(xsdp) => {
                let xsdp = unsafe { &**xsdp };
                if unsafe { !xsdp.validate() } {
                    return Err(AcpiError::InvalidChecksum);
                }
                let xsdt = unsafe { xsdp.xsdt() };
                if unsafe { !(&*xsdt).validate() } {
                    return Err(AcpiError::InvalidChecksum);
                }

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
                if index >= xsdt.len() {
                    return None;
                }
                let address: u64 = xsdt.tables[index];
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
