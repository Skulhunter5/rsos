use core::mem;

use crate::{
    io::{inl, inw, outl},
    println,
};

pub type VendorId = u16;
pub type DeviceId = u16;

#[derive(Debug, Clone, Copy)]
pub struct PciDevice {
    bus: u8,
    slot: u8,
    function: u8,
}

impl PciDevice {
    pub fn new(bus: u8, slot: u8, function: u8) -> Self {
        Self { bus, slot, function }
    }
}

pub struct Pci;

impl Pci {
    const PORT_CONFIG_ADDRESS: u16 = 0xCF8;
    const PORT_CONFIG_DATA: u16 = 0xCFC;

    const VENDOR_ID_DEVICE_DOES_NOT_EXIST: VendorId = 0xFFFF;
    
    pub fn get_vendor_id(device: PciDevice) -> VendorId {
        Self::config_read_u32(device.bus, device.slot, device.function, 0) as u16
    }

    pub fn get_device_id(device: PciDevice) -> DeviceId {
        (Self::config_read_u32(device.bus, device.slot, device.function, 0) >> (8 * mem::size_of::<u16>())) as u16
    }

    pub fn config_read_u16(bus: u8, slot: u8, function: u8, offset: u8) -> u16 {
        let address = 0x80000000u32
            | (offset as u32 & 0xFC)
            | ((function as u32) << 8)
            | ((slot as u32) << 11)
            | ((bus as u32) << 16);

        unsafe {
            outl(Self::PORT_CONFIG_ADDRESS, address);
        }
        let value = unsafe { inw(Self::PORT_CONFIG_DATA) };

        value
    }

    pub fn config_read_u32(bus: u8, slot: u8, function: u8, offset: u8) -> u32 {
        let address = 0x80000000u32
            | (offset as u32 & 0xFC)
            | ((function as u32 & 0b111) << 8)
            | ((slot as u32) << 11)
            | ((bus as u32) << 16);

        unsafe {
            outl(Self::PORT_CONFIG_ADDRESS, address);
        }
        let value = unsafe { inl(Self::PORT_CONFIG_DATA) };

        value
    }
}

#[allow(unused)]
pub unsafe fn test() {
    for i in 0..8 {
        let device = PciDevice::new(0, 31, i);
        let vendor_id = Pci::get_vendor_id(device);
        if vendor_id == Pci::VENDOR_ID_DEVICE_DOES_NOT_EXIST {
            continue;
        }
        let device_id = Pci::get_device_id(device);
        println!("{:?}: {:#x}:{:#x}", device, vendor_id, device_id);
    }
    //let device = PciDevice::new(0, 0, 0);
    //let a = (Pci::config_read_u32(device.bus, device.slot, device.function, 0) >> (8 * mem::size_of::<u16>())) as u16;
    //let b = Pci::config_read_u16(device.bus, device.slot, device.function, 0);
    //let c = Pci::config_read_u32(device.bus, device.slot, device.function, 0);
    //println!("a: {:#x}", a);
    //println!("b: {:#x}", b);
    //println!("c: {:#x}", c);
}
