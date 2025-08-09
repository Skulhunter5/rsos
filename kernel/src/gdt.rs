use core::{arch::asm, ptr};

use alloc::boxed::Box;

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

    pub fn for_tss(tss: &TaskStateSegment) -> (Self, Self) {
        let present = true;
        let privilege_level = PrivilegeLevel::Ring0;
        let ty = DescriptorType::SystemSegment;
        let ss_ty = 0x9;
        let access_byte = ((present as u8) << 7)
            | ((privilege_level as u8) << 5)
            | ((ty as u8) << 4) | ss_ty;
        let access = (access_byte as u64) << Self::OFFSET_ACCESS;
        let flags = (0x0 as u64) << Self::OFFSET_FLAGS;
        let address = ptr::from_ref(tss) as u64;
        let entry0 = {
            // lower half of address in entry0
            let base = address & 0xFFFFFFFF;
            let limit = size_of::<TaskStateSegment>() as u64 - 1;
            let val = (limit & 0xFFFF)
                | ((base & 0xFFFF) << 16)
                | ((base & 0xFF0000) << 16)
                | access
                | (limit & 0xF0000)
                | flags
                | ((base & 0xFF000000) << 32);
            Self(val)
        };
        let entry1 = {
            // higher half of address in lower half of entry1
            let base = (address >> 32) & 0xFFFFFFFF;
            let reserved = 0;
            let val = (reserved << 32) | base;
            Self(val)
        };
        (entry0, entry1)
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

const _: () = {
    assert!(size_of::<TaskStateSegment>() == 0x68);
};

#[derive(Debug, Clone, Copy)]
#[repr(C, packed(4))]
pub struct TaskStateSegment {
    reserved0: u32,
    rsp0: u64,
    rsp1: u64,
    rsp2: u64,
    reserved1: [u32; 2],
    ist1: u64,
    ist2: u64,
    ist3: u64,
    ist4: u64,
    ist5: u64,
    ist6: u64,
    ist7: u64,
    reserved2: [u32; 2],
    reserved3: u16,
    iopb: u16,
}

impl TaskStateSegment {
    pub const fn zero() -> Self {
        Self {
            reserved0: 0,
            rsp0: 0,
            rsp1: 0,
            rsp2: 0,
            reserved1: [0; 2],
            ist1: 0,
            ist2: 0,
            ist3: 0,
            ist4: 0,
            ist5: 0,
            ist6: 0,
            ist7: 0,
            reserved2: [0; 2],
            reserved3: 0,
            iopb: 0,
        }
    }
}

pub fn reload_segment_registers(kernel_code_ss: u16, kernel_data_ss: u16) {
    unsafe {
        asm!(
            "mov ds, ax",
            "mov es, ax",
            "mov fs, ax",
            "mov gs, ax",
            "mov ss, ax",
            in("ax") kernel_data_ss,
        );
        asm!(
            "push rax",
            "lea rax, [rip + 2f]",
            "push rax",
            "retfq",
            "2:",
            in("rax") kernel_code_ss,
            lateout("rax") _,
        );
    }
}

const _: () = {
    assert!(size_of::<[SegmentDescriptor; 4]>() == size_of::<SegmentDescriptor>() * 4);
};

#[derive(Debug)]
#[repr(transparent)]
pub struct GlobalDescriptorTable([SegmentDescriptor]);

impl GlobalDescriptorTable {
    pub fn new(len: usize) -> Box<Self> {
        let mut arr: Box<[SegmentDescriptor]> = unsafe { Box::new_zeroed_slice(len).assume_init() };
        let ptr = ptr::from_raw_parts_mut(arr.as_mut_ptr(), len);
        let gdt = unsafe { Box::from_raw(ptr) };

        gdt
    }

    pub fn set(&mut self, index: usize, descriptor: SegmentDescriptor) {
        self.0[index] = descriptor;
    }

    pub fn load(&self, kernel_code_ss: u16, kernel_data_ss: u16) {
        let gdtr = Gdtr::new_for(self);
        unsafe {
            asm!("lgdt [{}]", in(reg) ptr::from_ref(&gdtr), options(readonly, preserves_flags, nostack));
        }
        reload_segment_registers(kernel_code_ss, kernel_data_ss);
    }
}

#[repr(C, packed)]
struct Gdtr {
    size: u16,
    address: usize,
}

impl Gdtr {
    fn new_for(gdt: &GlobalDescriptorTable) -> Self {
        let size = (gdt.0.len() * size_of::<SegmentDescriptor>() - 1) as u16;
        let address = gdt.0.as_ptr() as usize;
        Self { size, address }
    }
}

pub fn init() -> (Box<GlobalDescriptorTable>, Box<TaskStateSegment>) {
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

    // todo!("add segment descriptor for tss");

    let tss = Box::new(TaskStateSegment::zero());

    let mut gdt = GlobalDescriptorTable::new(7);
    gdt.set(0, SD_NULL);
    gdt.set(1, SD_KERNEL_CODE);
    gdt.set(2, SD_KERNEL_DATA);
    gdt.set(3, SD_USER_CODE);
    gdt.set(4, SD_USER_DATA);
    let (tss_entry0, tss_entry1) = SegmentDescriptor::for_tss(tss.as_ref());
    gdt.set(5, tss_entry0);
    gdt.set(6, tss_entry1);
    gdt.load(0x08, 0x10);

    (gdt, tss)
}
