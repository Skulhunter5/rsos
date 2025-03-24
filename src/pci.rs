use crate::{
    io::{inl, outl},
    println,
};

const PCI_PORT_CONFIG_ADDRESS: u16 = 0xCF8;
const PCI_PORT_CONFIG_DATA: u16 = 0xCFC;

pub fn config_read_u32(bus: u8, slot: u8, function: u8, offset: u8) -> u32 {
    let address = 0x80000000u32
        | (offset as u32 & 0xFC)
        | ((function as u32) << 8)
        | ((slot as u32) << 11)
        | ((bus as u32) << 16);

    unsafe {
        outl(PCI_PORT_CONFIG_ADDRESS, address);
    }
    let value = unsafe { inl(PCI_PORT_CONFIG_DATA) };

    value
}

#[allow(unused)]
pub unsafe fn test() {
    let a = config_read_u32(1, 0, 0, 0);
    let b = config_read_u32(0, 0, 0, 4);
    let c = config_read_u32(200, 6, 0, 8);
    println!("a: {}", a);
    println!("a: {:#x}", a);
    println!("b: {}", b);
    println!("c: {}", c);
}
