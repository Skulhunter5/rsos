#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PhysicalAddress(pub u64);

impl core::fmt::Debug for PhysicalAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "PhysicalAddress({:x})", self.0)
    }
}

impl Into<u64> for PhysicalAddress {
    fn into(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct VirtualAddress(pub u64);

impl core::fmt::Debug for VirtualAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "VirtualAddress({:x})", self.0)
    }
}

impl Into<u64> for VirtualAddress {
    fn into(self) -> u64 {
        self.0
    }
}

// TODO: fix address mask to include "execute disable" bit etc.
#[repr(transparent)]
pub struct PageEntry(u64);

impl PageEntry {
    pub fn address(&self) -> u64 {
        self.0 & !0xFFF
    }

    pub fn present(&self) -> bool {
        self.0 & (1 << 0) != 0
    }

    pub fn writable(&self) -> bool {
        self.0 & (1 << 1) != 0
    }

    pub fn user_access(&self) -> bool {
        self.0 & (1 << 2) != 0
    }

    pub fn write_through(&self) -> bool {
        self.0 & (1 << 3) != 0
    }

    pub fn cache_disabled(&self) -> bool {
        self.0 & (1 << 4) != 0
    }

    pub fn cacheable(&self) -> bool {
        self.0 & (1 << 4) == 0
    }

    pub fn accessed(&self) -> bool {
        self.0 & (1 << 5) != 0
    }

    pub fn dirty(&self) -> bool {
        self.0 & (1 << 6) != 0
    }

    pub fn page_size(&self) -> bool {
        self.0 & (1 << 7) != 0
    }

    pub fn execute_disabled(&self) -> bool {
        self.0 & (1 << 63) != 0
    }
}

impl core::fmt::Debug for PageEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.0 == 0 {
            return write!(f, "0");
        }
        write!(f, "PageEntry {{ address: 0x{:x}, other: ?? }}", self.address())
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Pml4 {
    entries: [PageEntry; 512],
}

impl Pml4 {
    pub unsafe fn get_current() -> &'static mut Pml4 {
        let address = read_cr3().pml4_address();
        unsafe { Self::from_raw(address) }
    }

    pub unsafe fn from_raw(address: PhysicalAddress) -> &'static mut Pml4 {
        let address: u64 = address.into();
        unsafe { (address as *mut Pml4).as_mut().unwrap() }
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Cr3Value(u64);

impl Cr3Value {
    pub fn pml4_address(&self) -> PhysicalAddress {
        PhysicalAddress(self.0 & !0xFFF)
    }
}

impl core::fmt::Debug for Cr3Value {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Cr3Value {{ other: ??, pml4_address: {:?} }}", self.pml4_address())
    }
}

pub fn read_cr3() -> Cr3Value {
    let mut val;
    unsafe { core::arch::asm!("mov {}, cr3", out(reg) val, options(nostack, preserves_flags)); }
    Cr3Value(val)
}

pub fn write_cr3(val: Cr3Value) {
    let val = val.0;
    unsafe { core::arch::asm!("mov cr3, {}", in(reg) val, options(nostack, preserves_flags)); }
}
