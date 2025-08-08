use core::{arch::asm, ptr};

use alloc::{boxed::Box, vec};

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct SegmentDescriptor(u64);

impl SegmentDescriptor {
    const OFFSET_FLAGS: usize = 52;
    const OFFSET_ACCESS: usize = 40;

    pub const fn new(base: u32, limit: u32, flags: Flags, access: Access) -> Self {
        let base = base as u64;
        let limit = (limit & 0xFFFFF) as u64;
        let access = (access.0 as u64) << Self::OFFSET_ACCESS;
        let flags = (flags.0 as u64) << Self::OFFSET_FLAGS;
        let val = (limit & 0xFFFF)
            | ((base & 0xFFFF) << 16)
            | ((base & 0xFF0000) << 16)
            | access
            | (limit & 0xF0000)
            | flags
            | ((base & 0xFF000000) << 32);
        Self(val)
    }

    pub const fn zero() -> Self {
        Self(0)
    }

    pub const fn null() -> Self {
        Self(0)
    }

    pub const fn flags(&self) -> Flags {
        let val = (self.0 >> Self::OFFSET_FLAGS) & 0xF;
        Flags(val as u8)
    }

    pub const fn access(&self) -> Access {
        let val = (self.0 >> Self::OFFSET_ACCESS) & 0xFF;
        Access(val as u8)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Flags(u8);

impl Flags {
    pub const fn new(granularity: Granularity, size: Size, code_segment_64: bool) -> Self {
        let val = ((granularity as u8) << 3) | ((size as u8) << 2) | ((code_segment_64 as u8) << 1);
        Self(val)
    }

    pub const fn granularity(&self) -> Granularity {
        match (self.0 >> 3) & 1 != 0 {
            false => Granularity::Byte,
            true => Granularity::Page4KiB,
        }
    }

    pub const fn size(&self) -> Size {
        match (self.0 >> 2) & 1 != 0 {
            false => Size::ProtectedMode16,
            true => Size::ProtectedMode32,
        }
    }

    pub const fn code_segment_64(&self) -> bool {
        (self.0 >> 1) & 1 != 0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Access(u8);

impl Access {
    pub const fn new(
        present: bool,
        privilege_level: PrivilegeLevel,
        ty: DescriptorType,
        executable: bool,
        dc: bool,
        rw: bool,
        accessed: bool,
    ) -> Self {
        let val = ((present as u8) << 7)
            | ((privilege_level as u8) << 5)
            | ((ty as u8) << 4)
            | ((executable as u8) << 3)
            | ((dc as u8) << 2)
            | ((rw as u8) << 1)
            | (accessed as u8);
        Self(val)
    }

    pub const fn present(&self) -> bool {
        (self.0 >> 7) & 1 != 0
    }

    pub const fn privilege_level(&self) -> PrivilegeLevel {
        match (self.0 >> 5) & 0b11 {
            0 => PrivilegeLevel::Ring0,
            1 => PrivilegeLevel::Ring1,
            2 => PrivilegeLevel::Ring2,
            3 => PrivilegeLevel::Ring3,
            _ => unreachable!(),
        }
    }

    pub const fn ty(&self) -> DescriptorType {
        match (self.0 >> 4) & 1 != 0 {
            false => DescriptorType::SystemSegment,
            true => DescriptorType::CodeOrData,
        }
    }

    pub const fn executable(&self) -> bool {
        (self.0 >> 3) & 1 != 0
    }

    pub const fn dc(&self) -> bool {
        (self.0 >> 2) & 1 != 0
    }

    pub const fn rw(&self) -> bool {
        (self.0 >> 1) & 1 != 0
    }

    pub const fn accessed(&self) -> bool {
        self.0 & 1 != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PrivilegeLevel {
    Ring0 = 0,
    Ring1 = 1,
    Ring2 = 2,
    Ring3 = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescriptorType {
    SystemSegment = 0,
    CodeOrData = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Granularity {
    Byte = 0,
    Page4KiB = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
    ProtectedMode16 = 0,
    ProtectedMode32 = 1,
}

fn reload_segment_registers() {
    unsafe {
        asm!(
            "mov ax, {data_seg}",
            "mov ds, ax",
            "mov es, ax",
            "mov fs, ax",
            "mov gs, ax",
            "mov ss, ax",
            data_seg = const 0x10,
            out("ax") _,
        );
        asm!(
           "push {code_seg}",
           "lea rax, [rip + 2f]",
           "push rax",
           "retfq",
           "2:",
           code_seg = const 0x08,
           out("rax") _,
        );
    }
}

const _: () = {
    assert!(size_of::<[SegmentDescriptor; 4]>() == size_of::<SegmentDescriptor>() * 4);
};

#[repr(transparent)]
pub struct Gdt(Box<[SegmentDescriptor]>);

impl Gdt {
    pub fn new(table: Box<[SegmentDescriptor]>) -> Self {
        Self(table)
    }

    pub fn load(&self) {
        let gdtr = Gdtr::new_for(self);
        unsafe {
            asm!("lgdt [{}]", in(reg) ptr::from_ref(&gdtr), options(readonly, preserves_flags, nostack));
        }
        // TODO: check that this works as intended
        reload_segment_registers();
    }
}

#[repr(C, packed)]
struct Gdtr {
    size: u16,
    address: usize,
}

impl Gdtr {
    fn new_for(gdt: &Gdt) -> Self {
        let size = (gdt.0.len() * size_of::<SegmentDescriptor>() - 1) as u16;
        let address = gdt.0.as_ptr() as usize;
        Self { size, address }
    }
}

pub fn init() -> Gdt {
    const SD_NULL: SegmentDescriptor = SegmentDescriptor::null();
    const SD_KERNEL_CODE: SegmentDescriptor = SegmentDescriptor::new(
        0,
        0xFFFFF,
        Flags::new(Granularity::Page4KiB, Size::ProtectedMode16, true),
        Access::new(
            true,
            PrivilegeLevel::Ring0,
            DescriptorType::CodeOrData,
            true,
            false,
            true,
            false,
        ),
    );
    const SD_KERNEL_DATA: SegmentDescriptor = SegmentDescriptor::new(
        0,
        0xFFFFF,
        Flags::new(Granularity::Page4KiB, Size::ProtectedMode32, false),
        Access::new(
            true,
            PrivilegeLevel::Ring0,
            DescriptorType::CodeOrData,
            false,
            false,
            true,
            false,
        ),
    );
    const SD_USER_CODE: SegmentDescriptor = SegmentDescriptor::new(
        0,
        0xFFFFF,
        Flags::new(Granularity::Page4KiB, Size::ProtectedMode16, true),
        Access::new(
            true,
            PrivilegeLevel::Ring3,
            DescriptorType::CodeOrData,
            true,
            false,
            true,
            false,
        ),
    );
    const SD_USER_DATA: SegmentDescriptor = SegmentDescriptor::new(
        0,
        0xFFFFF,
        Flags::new(Granularity::Page4KiB, Size::ProtectedMode32, false),
        Access::new(
            true,
            PrivilegeLevel::Ring3,
            DescriptorType::CodeOrData,
            false,
            false,
            true,
            false,
        ),
    );
    // static DEFAULT_GDT: [SegmentDescriptor; 5] = [
    //     SD_NULL,
    //     SD_KERNEL_CODE,
    //     SD_KERNEL_DATA,
    //     SD_USER_CODE,
    //     SD_USER_DATA,
    // ];

    // todo!("add segment descriptor for tss");

    let entries = vec![
        SD_NULL,
        SD_KERNEL_CODE,
        SD_KERNEL_DATA,
        SD_USER_CODE,
        SD_USER_DATA,
    ]
    .into_boxed_slice();
    let gdt = Gdt::new(entries);
    gdt.load();

    gdt
}
