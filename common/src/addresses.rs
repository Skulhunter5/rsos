pub type PAddr = PhysicalAddress;
pub type VAddr = VirtualAddress;
pub type PhysAddr = PhysicalAddress;
pub type VirtAddr = VirtualAddress;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PhysicalAddress(pub u64);

impl PhysicalAddress {
    pub fn null() -> Self {
        Self(0)
    }

    pub fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl core::fmt::Debug for PhysicalAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "PhysicalAddress(0x{:x})", self.0)
    }
}

impl Into<u64> for PhysicalAddress {
    fn into(self) -> u64 {
        self.0
    }
}

impl From<u64> for PhysicalAddress {
    fn from(address: u64) -> Self {
        Self(address)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct VirtualAddress(pub u64);

impl core::fmt::Debug for VirtualAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "VirtualAddress(0x{:x})", self.0)
    }
}

impl Into<u64> for VirtualAddress {
    fn into(self) -> u64 {
        self.0
    }
}

impl From<u64> for VirtualAddress {
    fn from(address: u64) -> Self {
        Self(address)
    }
}
