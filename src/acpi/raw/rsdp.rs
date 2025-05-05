use core::mem;

use super::{Rsdt, SdtHeader, Xsdt};

const RSDP_SIZE: usize = mem::size_of::<Rsdp>();

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Rsdp {
    pub signature: [u8; 8],
    pub checksum: u8,
    pub oem_id: [u8; 6],
    pub revision: u8,
    pub rsdt_address: u32,
}

impl Rsdp {
    pub unsafe fn validate(&self) -> bool {
        let rsdp_ptr = self as *const Self;
        let rsdp_ptr = rsdp_ptr as *const u8;

        let sum = (0..RSDP_SIZE).map(|i| unsafe { *rsdp_ptr.add(i) } as usize).sum::<usize>();
        return sum & 0xFF == 0;
    }

    pub unsafe fn rsdt(&self) -> *const Rsdt {
        let rsdp = self;
        let address = rsdp.rsdt_address;
        let header = unsafe { &*(address as *const SdtHeader) };

        let content_size = (header.length as usize) - mem::size_of::<SdtHeader>();
        let count = content_size / mem::size_of::<u32>();
        let ptr: *const Rsdt = core::ptr::from_raw_parts(address as *const (), count);

        ptr
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Xsdp {
    pub rsdp: Rsdp,
    pub length: u32,
    pub xsdt_address: u64,
    pub extended_checksum: u8,
    pub reserved: [u8; 3],
}

impl Xsdp {
    pub unsafe fn validate(&self) -> bool {
        if unsafe { !self.rsdp.validate() } {
            return false;
        }

        const XSDP_SIZE: usize = mem::size_of::<Xsdp>() - RSDP_SIZE;

        let rsdp_ptr = self as *const Self;
        let rsdp_ptr = rsdp_ptr as *const u8;
        let xsdp_ptr = unsafe { rsdp_ptr.add(RSDP_SIZE) };

        let sum = (0..XSDP_SIZE).map(|i| unsafe { *xsdp_ptr.add(i) } as usize).sum::<usize>();
        return sum & 0xFF == 0;
    }

    pub unsafe fn rsdt(&self) -> *const Rsdt {
        unsafe { self.rsdp.rsdt() }
    }

    pub unsafe fn xsdt(&self) -> *const Xsdt {
        let xsdp = self;
        let address = xsdp.xsdt_address;
        let header = unsafe { &*(address as *const SdtHeader) };

        let content_size = (header.length as usize) - mem::size_of::<SdtHeader>();
        let count = content_size / mem::size_of::<u64>();
        let ptr: *const Xsdt = core::ptr::from_raw_parts(address as *const (), count);

        ptr
    }
}
