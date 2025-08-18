use core::{arch::asm, ptr};

#[derive(Debug, Clone, Copy)]
#[repr(C, packed(4))]
pub struct IdtEntry {
    isr_low: u16,
    pub kernel_cs: u16,
    pub ist: u8,
    pub attributes: u8,
    isr_mid: u16,
    isr_high: u32,
    reserved: u32,
}

impl IdtEntry {
    pub const fn new(isr: u64, kernel_cs: u16, ist: u8, attributes: u8) -> Self {
        let isr_low = isr as u16;
        let isr_mid = (isr >> 16) as u16;
        let isr_high = (isr >> 32) as u32;
        Self {
            isr_low,
            kernel_cs,
            ist,
            attributes,
            isr_mid,
            isr_high,
            reserved: 0,
        }
    }

    pub const fn zero() -> Self {
        Self {
            isr_low: 0,
            kernel_cs: 0,
            ist: 0,
            attributes: 0,
            isr_mid: 0,
            isr_high: 0,
            reserved: 0,
        }
    }

    pub const fn isr(&self) -> u64 {
        (self.isr_low as u64) | ((self.isr_mid as u64) << 16) | ((self.isr_high as u64) << 32)
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct InterruptDescriptorTable([IdtEntry; 256]);

impl InterruptDescriptorTable {
    pub const fn zero() -> Self {
        Self([IdtEntry::zero(); 256])
    }

    pub fn load(&self) {
        let idtr = Idtr::new_for(self);
        unsafe {
            // asm!(
            //     "cli",
            //     "lidt [{}]",
            //     "sti",
            //     in(reg) ptr::from_ref(&idtr) as usize
            // );
            asm!(
                "lidt [{}]",
                in(reg) ptr::from_ref(&idtr) as usize
            );
        }
    }
}

impl core::ops::Index<usize> for InterruptDescriptorTable {
    type Output = IdtEntry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl core::ops::IndexMut<usize> for InterruptDescriptorTable {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

#[repr(C)]
struct Idtr {
    limit: u16,
    base: usize,
}

impl Idtr {
    fn new_for(idt: &InterruptDescriptorTable) -> Self {
        Self {
            limit: (size_of::<InterruptDescriptorTable>() - 1) as u16,
            base: ptr::from_ref(idt) as usize,
        }
    }
}
