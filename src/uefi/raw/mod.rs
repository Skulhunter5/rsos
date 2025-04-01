use core::ffi::c_void;

pub mod protocols;
pub mod services;
pub mod tables;

pub type ImageHandle = *const c_void;

#[repr(C)]
#[derive(Debug)]
pub struct MemoryDescriptor {
    ty: u32,
    physical_start: *const c_void,
    virtual_start: *const c_void,
    page_count: u64,
    attribute: MemoryAttribute,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct MemoryAttribute(u64);

impl MemoryAttribute {
    // Can be configured as not cacheable
    pub const UC: Self = Self(0x1);
    // Can be configured as write combining
    pub const WC: Self = Self(0x2);
    // Can be configured as cacheable with "write through" policy
    pub const WT: Self = Self(0x4);
    // Can be configured as cacheable with "write back" policy
    pub const WB: Self = Self(0x8);
    // Can be configured as not cacheable, exported and supports the "fetch and add" semaphore
    // mechanism
    pub const UCE: Self = Self(0x10);
    // Can be configured as write-protected by system hardware
    pub const WP: Self = Self(0x1000);
    // Can be configured as read-protected by system hardware
    pub const RP: Self = Self(0x2000);
    // Can be configured as exec-protected by system hardware
    pub const XP: Self = Self(0x4000);
    // Persistent Memory
    pub const NV: Self = Self(0x8000);
    // More reliable than other memory in the system
    pub const MORE_RELIABLE: Self = Self(0x10000);
    // Can be configured as read-only by system hardware
    pub const RO: Self = Self(0x20000);
    // Specific-purpose memory (SPM)
    pub const SP: Self = Self(0x40000);
    // Can be protected by the CPU's memory cryptographic capabilities
    pub const CPU_CRYPTO: Self = Self(0x80000);
    // Can be dynamically removed at runtime
    pub const HOT_PLUGGABLE: Self = Self(0x100000);
    // Needs to be given a virtual mapping by the OS when calling SetVirtualAddressMap(...)
    pub const RUNTIME: Self = Self(0x8000000000000000);
    // There are additional ISA-specific memory attributes
    pub const ISA_VALID: Self = Self(0x4000000000000000);
    // Bit-mask for ISA-specific memory attributes
    pub const ISA_MASK: Self = Self(0x0FFFF00000000000);
}

impl MemoryAttribute {
    #[inline(always)]
    pub fn contains(self, other: MemoryAttribute) -> bool {
        self.0 & other.0 != 0
    }
}

impl core::ops::BitOr for MemoryAttribute {
    type Output = Self;

    fn bitor(self, other: Self) -> Self::Output {
        MemoryAttribute(self.0 | other.0)
    }
}
