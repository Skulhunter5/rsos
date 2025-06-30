use core::ffi::c_void;

pub mod protocols;
pub mod services;
pub mod tables;

pub type ImageHandle = *const c_void;

#[macro_export]
macro_rules! newtype_enum {
    (
        $(#[$type_attrs:meta])*
        $visibility:vis enum $type:ident : $base_integer:ty => $(#[$impl_attrs:meta])* {
            $(
                $(#[$variant_attrs:meta])*
                $variant:ident = $value:expr,
            )*
        }
    ) => {
        $(#[$type_attrs])*
        #[repr(transparent)]
        #[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
        $visibility struct $type(pub $base_integer);

        $(#[$impl_attrs])*
        #[allow(unused)]
        impl $type {
            $(
                $(#[$variant_attrs])*
                pub const $variant: $type = $type($value);
            )*
        }

        impl core::fmt::Debug for $type {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                match *self {
                    // Display variants by their name, like Rust enums do
                    $(
                        $type::$variant => write!(f, stringify!($variant)),
                    )*

                    // Display unknown variants in tuple struct format
                    $type(unknown) => {
                        write!(f, "{}({})", stringify!($type), unknown)
                    }
                }
            }
        }
    }
}

newtype_enum! {
    #[derive(Default)]
    pub enum MemoryType: u32 => #[allow(missing_docs)] {
        RESERVED_MEMORY_TYPE = 0,
        LOADER_CODE = 1,
        LOADER_DATA = 2,
        BOOT_SERVICES_CODE = 3,
        BOOT_SERVICES_DATA = 4,
        RUNTIME_SERVICES_CODE = 5,
        RUNTIME_SERVICES_DATA = 6,
        CONVENTIONAL_MEMORY = 7,
        UNUSABLE_MEMORY = 8,
        ACPI_RECLAIM_MEMORY = 9,
        ACPI_MEMORY_NVS = 10,
        MEMORY_MAPPED_IO = 11,
        MEMORY_MAPPED_IO_PORT_SPACE = 12,
        PAL_CODE = 13,
        PERSISTENT_MEMORY = 14,
        UNACCEPTED_MEMORY_TYPE = 15,
        MAX_MEMORY_TYPE = 16,
    }
}

//#[repr(u32)]
//#[derive(Debug)]
//pub enum MemoryType {
//    ReservedMemoryType,
//    LoaderCode,
//    LoaderData,
//    BootServicesCode,
//    BootServicesData,
//    RuntimeServicesCode,
//    RuntimeServicesData,
//    ConventionalMemory,
//    UnusableMemory,
//    ACPIReclaimMemory,
//    ACPIMemoryNVS,
//    MemoryMappedIO,
//    MemoryMappedIOPortSpace,
//    PalCode,
//    PersistentMemory,
//    UnacceptedMemoryType,
//    MaxMemoryType,
//}

#[derive(Debug, Clone, Copy)]
#[repr(C, align(16))]
pub struct MemoryDescriptor {
    pub ty: MemoryType,
    pad0: u32,
    pub physical_start: u64,
    pub virtual_start: u64,
    pub page_count: u64,
    pub attribute: MemoryAttribute,
}

impl MemoryDescriptor {
    pub fn zero() -> Self {
        Self {
            ty: MemoryType(0),
            pad0: 0,
            physical_start: 0,
            virtual_start: 0,
            page_count: 0,
            attribute: MemoryAttribute(0),
        }
    }
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
