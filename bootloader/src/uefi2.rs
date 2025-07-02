use core::{
    ptr,
    sync::atomic::{AtomicPtr, Ordering},
};

use bootloader::spin::Mutex;
use raw::Handle;

static IMAGE_HANDLE: AtomicPtr<()> = AtomicPtr::new(ptr::null_mut());

pub fn init(handle: Handle, system_table: *mut raw::SystemTable) {
    set_image_handle(handle);
    set_system_table(system_table);
}

pub fn set_image_handle(handle: Handle) {
    IMAGE_HANDLE.store(handle, Ordering::Release);
}

fn image_handle() -> Handle {
    let handle = IMAGE_HANDLE.load(Ordering::Acquire);
    if handle.is_null() {
        panic!("global image handle not set");
    }
    handle
}

static SYSTEM_TABLE: Mutex<AtomicPtr<raw::SystemTable>> =
    Mutex::new(AtomicPtr::new(ptr::null_mut()));

pub fn set_system_table(system_table: *mut raw::SystemTable) {
    SYSTEM_TABLE.lock().store(system_table, Ordering::Release);
}

pub fn with_system_table<F, R>(f: F) -> R
where
    F: FnOnce(&mut raw::SystemTable) -> R,
{
    let system_table = unsafe {
        SYSTEM_TABLE
            .lock()
            .load(Ordering::Acquire)
            .as_mut()
            .expect("global system table is not set")
    };
    f(system_table)
}

pub fn with_boot_services<F, R>(f: F) -> R
where
    F: FnOnce(&mut raw::BootServices) -> R,
{
    let boot_services = unsafe {
        &mut *SYSTEM_TABLE
            .lock()
            .load(Ordering::Acquire)
            .as_mut()
            .expect("global system table is not set")
            .boot_services
    };
    f(boot_services)
}

pub mod boot {
    use alloc::vec;

    use super::raw::{AllocateType, MemoryType};
    use super::with_boot_services;

    use super::{
        image_handle,
        memory_map::MemoryMap,
        raw::{MemoryDescriptor, Status},
    };

    pub fn allocate_pages(
        allocate_type: AllocateType,
        memory_type: MemoryType,
        pages: usize,
        address_hint: u64,
    ) -> Option<u64> {
        let mut address = address_hint;
        let status = with_boot_services(|boot_services| {
            (boot_services.allocate_pages)(allocate_type, memory_type, pages, &mut address)
        });
        if status != Status::SUCCESS {
            return None;
        }

        if address != 0 {
            return Some(address);
        }

        let mut address = address_hint;
        let status = with_boot_services(|boot_services| {
            (boot_services.allocate_pages)(allocate_type, memory_type, pages, &mut address)
        });
        if status != Status::SUCCESS {
            return None;
        }

        assert!(address != 0);

        todo!();
    }

    pub fn free_pages(address: u64, pages: usize) {
        let status = with_boot_services(|boot_services| (boot_services.free_pages)(address, pages));
        // TODO: figure out what to do in case the free fails
        if status != Status::SUCCESS {
            todo!("figure out what to do in this code branch");
        }
    }

    pub fn get_memory_map_size() -> Option<usize> {
        let mut size = 0;
        let mut key = 0;
        let mut descriptor_size = 0;
        let mut descriptor_version = 0;

        let status = with_boot_services(|boot_services| {
            (boot_services.get_memory_map)(
                &mut size,
                core::ptr::null_mut(),
                &mut key,
                &mut descriptor_size,
                &mut descriptor_version,
            )
        });

        if status == Status::BUFFER_TOO_SMALL {
            return Some(size);
        } else {
            return None;
        }
    }

    // TODO: retry getting the memory map if it doesn't succeed
    // TODO: clean this up to be more readable
    pub fn get_memory_map() -> MemoryMap {
        let required_size = get_memory_map_size().unwrap();
        assert!(required_size % size_of::<MemoryDescriptor>() == 0);

        let mut buffer =
            vec![MemoryDescriptor::zero(); required_size / size_of::<MemoryDescriptor>()];
        assert!(buffer.as_ptr() as usize % align_of::<MemoryDescriptor>() == 0);

        let buffer_size = buffer.len() * size_of::<MemoryDescriptor>();
        assert!(buffer_size == required_size);

        let mut size = buffer_size;
        let mut key = 0;
        let mut descriptor_size = 0;
        let mut descriptor_version = 0;

        let status = with_boot_services(|boot_services| {
            (boot_services.get_memory_map)(
                &mut size,
                buffer.as_mut_ptr(),
                &mut key,
                &mut descriptor_size,
                &mut descriptor_version,
            )
        });

        let resulting_size = size;
        assert!(resulting_size <= required_size);

        assert!(status != Status::BUFFER_TOO_SMALL);
        if status != Status::SUCCESS {
            panic!("failed to get memory map");
        }

        assert!(resulting_size % size_of::<MemoryDescriptor>() == 0);
        buffer.resize(
            resulting_size / size_of::<MemoryDescriptor>(),
            MemoryDescriptor::zero(),
        );
        buffer.shrink_to_fit();
        let map = buffer.into_boxed_slice();

        MemoryMap::new(key, map)
    }

    #[must_use]
    pub unsafe fn exit_boot_services() -> MemoryMap {
        // Try twice, otherwise panic
        for _ in 0..2 {
            let memory_map = get_memory_map();

            let handle = image_handle();

            let status = with_boot_services(|boot_services| {
                (boot_services.exit_boot_services)(handle, memory_map.get_key())
            });

            if status == Status::SUCCESS {
                return memory_map;
            }
        }

        panic!("failed to exit boot services");
    }
}

pub mod raw {
    pub type Handle = *mut ();

    #[repr(C)]
    pub struct TableHeader {
        pub signature: u64,
        pub revision: u32,
        pub size: u32,
        pub crc32: u32,
        pub reserved: u32,
    }

    #[repr(C)]
    pub struct SystemTable {
        pub header: TableHeader,
        pub firmware_vendor: *const (),
        pub firmware_revision: u32,
        pub console_in_handle: Handle,
        pub con_in: *const (),
        pub console_out_handle: Handle,
        pub con_out: *const (),
        pub standard_error_handle: Handle,
        pub std_err: *const (),
        pub runtime_services: *const (),
        pub boot_services: &'static mut BootServices,
        pub number_of_table_entries: usize,
        pub configuration_table: *mut ConfigurationTableEntry,
    }

    #[repr(C)]
    pub struct ConfigurationTableEntry {
        pub guid: Guid,
        pub ptr: *mut (),
    }

    #[repr(C)]
    #[derive(Debug, PartialEq, Eq)]
    pub struct Guid {
        time_low: u32,
        time_mid: u16,
        time_high_and_revision: u16,
        clock_seq_high_and_reserved: u8,
        clock_seq_low: u8,
        node: [u8; 6],
    }

    impl Guid {
        // EFI_ACPI_TABLE_GUID:
        // [0x8868e871, 0xe4f1, 0x11d3, 0xbc, 0x22, 0x0, 0x80, 0xc7, 0x3c, 0x88, 0x81]
        pub const EFI_ACPI_TABLE_GUID: Guid = Guid::new(
            0x8868e871,
            0xe4f1,
            0x11d3,
            0xbc,
            0x22,
            [0x0, 0x80, 0xc7, 0x3c, 0x88, 0x81],
        );

        pub const fn new(
            time_low: u32,
            time_mid: u16,
            time_high_and_revision: u16,
            clock_seq_high_and_reserved: u8,
            clock_seq_low: u8,
            node: [u8; 6],
        ) -> Self {
            Self {
                time_low,
                time_mid,
                time_high_and_revision,
                clock_seq_high_and_reserved,
                clock_seq_low,
                node,
            }
        }
    }

    #[repr(C)]
    pub struct BootServices {
        pub header: TableHeader,
        // Task Priority Services
        pub raise_tpl: *const (),
        pub restore_tpl: *const (),
        // Memory Services
        pub allocate_pages: AllocatePages,
        pub free_pages: FreePages,
        pub get_memory_map: GetMemoryMap,
        pub allocate_pool: *const (),
        pub free_pool: *const (),
        // Event & Timer Services
        pub create_event: *const (),
        pub set_timer: *const (),
        pub wait_for_event: *const (),
        pub signal_event: *const (),
        pub close_event: *const (),
        pub check_event: *const (),
        // Protocol Handler Services
        pub install_protocol_interface: *const (),
        pub reinstall_protocol_interface: *const (),
        pub uninstall_protocol_interface: *const (),
        pub handle_protocol: *const (),
        reserved0: *const (),
        pub register_protocol_notify: *const (),
        pub locate_handle: *const (),
        pub locate_device_path: *const (),
        pub install_configuration_table: *const (),
        // Image Services
        pub load_image: *const (),
        pub start_image: *const (),
        pub exit: *const (),
        pub unload_image: *const (),
        pub exit_boot_services: ExitBootServices,
        // Miscellaneous Services
        pub get_next_monotonic_count: *const (),
        pub stall: Stall,
        pub set_watchdog_timer: *const (),
        // TODO: add the remaining fields and version support
    }

    pub type AllocatePages = extern "efiapi" fn(
        allocate_type: AllocateType,
        memory_type: MemoryType,
        page_count: usize,
        ptr: &mut u64,
    ) -> Status;
    pub type FreePages = extern "efiapi" fn(address: u64, pages: usize) -> Status;

    pub type GetMemoryMap = extern "efiapi" fn(
        size: &mut usize,
        buffer: *mut MemoryDescriptor,
        key: &mut usize,
        descriptor_size: &mut usize,
        descriptor_version: &mut u32,
    ) -> Status;

    pub type ExitBootServices = extern "efiapi" fn(handle: Handle, map_key: usize) -> Status;

    pub type Stall = extern "efiapi" fn(microseconds: usize) -> Status;

    #[derive(Debug, Clone, Copy)]
    #[repr(u32)]
    pub enum AllocateType {
        AllocateAnyPages = 0,
        AllocateMaxAddress = 1,
        AllocateAddress = 2,
    }

    #[must_use = "this `Status` may be an error, which should be handled"]
    #[derive(PartialEq, Eq)]
    #[repr(C)]
    pub struct Status(usize);

    impl Status {
        // Success
        pub const SUCCESS: Self = Self(0);
        // EFI Errors
        pub const LOAD_ERROR: Self = Self::new_efi_error(1);
        pub const INVALID_PARAMETER: Self = Self::new_efi_error(2);
        pub const UNSUPPORTED: Self = Self::new_efi_error(3);
        pub const BAD_BUFFER_SIZE: Self = Self::new_efi_error(4);
        pub const BUFFER_TOO_SMALL: Self = Self::new_efi_error(5);
        // TODO: add remaining specified statuses from UEFI spec

        pub const HIGHEST_BIT: usize = 1 << (8 * size_of::<usize>() - 1);

        #[inline(always)]
        const fn new_efi_error(code: usize) -> Self {
            Self(Self::HIGHEST_BIT | code)
        }

        #[inline(always)]
        fn highest_bit(&self) -> usize {
            self.0 >> (8 * size_of::<usize>() - 1)
        }

        #[inline(always)]
        fn two_highest_bits(&self) -> usize {
            self.0 >> (8 * size_of::<usize>() - 2)
        }

        #[inline]
        pub fn is_error(&self) -> bool {
            self.highest_bit() == 0b1
        }

        #[inline]
        pub fn is_efi_error(&self) -> bool {
            self.two_highest_bits() == 0b10
        }

        #[inline]
        pub fn is_oem_error(&self) -> bool {
            self.two_highest_bits() == 0b11
        }

        #[inline]
        pub fn is_warning(&self) -> bool {
            self.highest_bit() == 0b0 && *self != Self::SUCCESS
        }

        #[inline]
        pub fn is_efi_warning(&self) -> bool {
            self.two_highest_bits() == 0b00 && *self != Self::SUCCESS
        }

        #[inline]
        pub fn is_oem_warning(&self) -> bool {
            self.two_highest_bits() == 0b01 && *self != Self::SUCCESS
        }

        #[inline]
        pub fn is_success(&self) -> bool {
            self.highest_bit() == 0b0
        }
    }

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
}

pub mod memory_map {
    use alloc::boxed::Box;
    use core::ops::Index;

    #[derive(Debug)]
    pub struct MemoryMap {
        key: usize,
        data: Box<[MemoryDescriptor]>,
    }

    impl MemoryMap {
        pub fn new(key: usize, data: Box<[MemoryDescriptor]>) -> Self {
            Self { key, data }
        }

        pub fn get_key(&self) -> usize {
            self.key
        }

        pub fn len(&self) -> usize {
            self.data.len()
        }

        pub fn iter(&self) -> core::slice::Iter<'_, MemoryDescriptor> {
            self.data.iter()
        }
    }

    impl Index<usize> for MemoryMap {
        type Output = MemoryDescriptor;

        fn index(&self, index: usize) -> &Self::Output {
            &self.data[index]
        }
    }

    macro_rules! range_index_impl {
        ( $($name:ident),* $(,)? ) => {
            $(
                use core::ops::$name;
                impl Index<$name<usize>> for MemoryMap {
                    type Output = [MemoryDescriptor];

                    fn index(&self, index: $name<usize>) -> &Self::Output {
                        &self.data[index]
                    }
                }
            )*
        }
    }

    range_index_impl!(Range, RangeInclusive, RangeTo, RangeToInclusive, RangeFrom);

    use core::ops::RangeFull;

    use super::raw::MemoryDescriptor;
    impl Index<RangeFull> for MemoryMap {
        type Output = [MemoryDescriptor];

        fn index(&self, index: RangeFull) -> &Self::Output {
            &self.data[index]
        }
    }
}
