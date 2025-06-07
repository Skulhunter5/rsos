use core::mem;

use crate::io::{inl, inw, outl};

mod r#type;
pub use r#type::*;
#[allow(non_snake_case)]
pub mod SubClass;

pub type VendorId = u16;
pub type DeviceId = u16;

#[derive(Debug, Clone, Copy)]
pub enum PciError {
    InvalidDeviceType {
        class: u8,
        subclass: u8,
        prog_if: u8,
    },
    DeviceDoesNotExist(PciAddress),
}

#[derive(Debug, Clone, Copy)]
pub struct PciAddress {
    bus: u8,
    device: u8,
    function: u8,
}

impl PciAddress {
    pub fn new(bus: u8, device: u8, function: u8) -> Self {
        Self {
            bus,
            device,
            function,
        }
    }
}

#[derive(Debug)]
pub struct PciDevice {
    address: PciAddress,
}

impl PciDevice {
    pub unsafe fn claim(address: PciAddress) -> Result<Self, PciError> {
        let device = Self { address };
        if !device.exists() {
            return Err(PciError::DeviceDoesNotExist(address));
        }

        Ok(device)
    }

    pub fn exists(&self) -> bool {
        self.vendor_id() != Pci::VENDOR_ID_DEVICE_DOES_NOT_EXIST
    }

    fn config_read_u32(&self, offset: u8) -> u32 {
        unsafe { Pci::config_read_u32(self.address, offset) }
    }

    fn config_write_u32(&mut self, offset: u8, value: u32) {
        unsafe { Pci::config_write_u32(self.address, offset, value) }
    }

    pub fn vendor_id(&self) -> VendorId {
        unsafe { Pci::get_vendor_id(self.address) }
    }

    pub fn device_id(&self) -> DeviceId {
        unsafe { Pci::get_device_id(self.address) }
    }

    pub fn device_type(&self) -> Result<DeviceType, PciError> {
        unsafe { Pci::get_device_type(self.address) }.map_err(|(class, subclass, prog_if)| {
            PciError::InvalidDeviceType {
                class,
                subclass,
                prog_if,
            }
        })
    }

    pub fn bus_mastering(&self) -> bool {
        const BUS_MASTERING_BIT: u32 = 1 << 2;
        self.config_read_u32(0x4) & BUS_MASTERING_BIT != 0
    }

    pub fn enable_bus_mastering(&mut self) {
        const BUS_MASTERING_BIT: u32 = 1 << 2;
        let old = self.config_read_u32(0x4);
        let new = old | BUS_MASTERING_BIT;
        self.config_write_u32(0x4, new);
    }

    pub fn disable_bus_mastering(&mut self) {
        const BUS_MASTERING_BIT: u32 = 1 << 2;
        let old = self.config_read_u32(0x4);
        let new = old & !BUS_MASTERING_BIT;
        self.config_write_u32(0x4, new);
    }

    pub fn bar0(&self) -> u32 {
        self.config_read_u32(0x10)
    }

    pub fn bar1(&self) -> u32 {
        self.config_read_u32(0x14)
    }

    pub fn bar2(&self) -> u32 {
        self.config_read_u32(0x18)
    }

    pub fn bar3(&self) -> u32 {
        self.config_read_u32(0x1C)
    }

    pub fn bar4(&self) -> u32 {
        self.config_read_u32(0x20)
    }

    pub fn bar5(&self) -> u32 {
        self.config_read_u32(0x24)
    }
}

pub struct Pci;

impl Pci {
    const PORT_CONFIG_ADDRESS: u16 = 0xCF8;
    const PORT_CONFIG_DATA: u16 = 0xCFC;

    const VENDOR_ID_DEVICE_DOES_NOT_EXIST: VendorId = 0xFFFF;

    unsafe fn device_exists(addr: PciAddress) -> bool {
        let vendor_id = unsafe { Self::get_vendor_id(addr) };
        vendor_id != Self::VENDOR_ID_DEVICE_DOES_NOT_EXIST
    }

    unsafe fn get_vendor_id(addr: PciAddress) -> VendorId {
        unsafe { Self::config_read_u32(addr, 0) as u16 }
    }

    unsafe fn get_device_id(addr: PciAddress) -> DeviceId {
        (unsafe { Self::config_read_u32(addr, 0) }
            >> (8 * mem::size_of::<u16>())) as u16
    }

    unsafe fn get_device_type(addr: PciAddress) -> Result<DeviceType, (u8, u8, u8)> {
        let val = unsafe { Self::config_read_u32(addr, 0x8) };
        let class_code = (val >> 24 & 0xFF) as u8;
        let subclass_code = (val >> 16 & 0xFF) as u8;
        let prog_if = (val >> 8 & 0xFF) as u8;

        DeviceType::try_from(class_code, subclass_code, prog_if)
    }

    unsafe fn config_read_u32(addr: PciAddress, offset: u8) -> u32 {
        let address = 0x80000000u32
            | (offset as u32 & 0xFC)
            | ((addr.function as u32 & 0b111) << 8)
            | ((addr.device as u32) << 11)
            | ((addr.bus as u32) << 16);

        unsafe {
            outl(Self::PORT_CONFIG_ADDRESS, address);
        }
        let value = unsafe { inl(Self::PORT_CONFIG_DATA) };

        value
    }

    unsafe fn config_write_u32(addr: PciAddress, offset: u8, value: u32) {
        let address = 0x80000000u32
            | (offset as u32 & 0xFC)
            | ((addr.function as u32 & 0b111) << 8)
            | ((addr.device as u32) << 11)
            | ((addr.bus as u32) << 16);

        unsafe {
            outl(Self::PORT_CONFIG_ADDRESS, address);
        }
        unsafe { outl(Self::PORT_CONFIG_DATA, value) };
    }
}

pub unsafe fn enumerate() -> PciEnumeration {
    PciEnumeration {
        bus: 0,
        device: 0,
        function: 0,
        done: false,
    }
}

#[derive(Debug)]
pub struct PciEnumeration {
    bus: u8,
    device: u8,
    function: u8,
    done: bool,
}

impl Iterator for PciEnumeration {
    type Item = PciDevice;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.done {
            let address = PciAddress::new(self.bus, self.device, self.function);
            self.function += 1;
            if self.function >= 8 {
                self.function = 0;
                self.device += 1;
            }
            if self.device >= 32 {
                self.device = 0;
                match self.bus.checked_add(1) {
                    Some(val) => self.bus = val,
                    None => self.done = true,
                }
            }
            if unsafe { Pci::device_exists(address) } {
                if let Ok(device) = unsafe { PciDevice::claim(address) } {
                    return Some(device);
                }
            }
        }

        None
    }
}
