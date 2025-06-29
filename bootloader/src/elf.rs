use alloc::{format, string::String, vec::Vec};

use crate::cursor::Cursor;

fn read_u16(cur: &mut Cursor, data_encoding: DataEncoding) -> u16 {
    match data_encoding {
        DataEncoding::Lsb => cur.read_u16_le(),
        DataEncoding::Msb => cur.read_u16_be(),
    }
}

fn read_u32(cur: &mut Cursor, data_encoding: DataEncoding) -> u32 {
    match data_encoding {
        DataEncoding::Lsb => cur.read_u32_le(),
        DataEncoding::Msb => cur.read_u32_be(),
    }
}

fn read_u64(cur: &mut Cursor, data_encoding: DataEncoding) -> u64 {
    match data_encoding {
        DataEncoding::Lsb => cur.read_u64_le(),
        DataEncoding::Msb => cur.read_u64_be(),
    }
}

#[derive(Debug)]
pub struct Elf {
    pub class: Class,
    pub data_encoding: DataEncoding,
    pub ty: ObjectFileType,
    pub machine: MachineArchitecture,
    pub entry: u64,
    pub program_headers: Vec<ProgramHeader>,
    pub section_headers: Vec<SectionHeader>,
}

impl Elf {
    pub fn parse(buffer: &[u8]) -> Result<Self, ParseElfError> {
        let mut cur = Cursor::new(buffer);
        if cur.read_bytes(4) != &[0x7f, b'E', b'L', b'F'] {
            return Err(ParseElfError::MissingMagicNumber);
        }

        let class = match cur.read_u8() {
            1 => Class::Elf32,
            2 => Class::Elf64,
            x => return Err(ParseElfError::InvalidElfClass(x)),
        };

        #[cfg(target_pointer_width = "64")]
        if class != Class::Elf64 {
            panic!("unsupported operation: elf class must be 64-bit");
        }
        #[cfg(not(target_pointer_width = "64"))]
        compile_error!("target_pointer_width != 64 not supported");

        let data_encoding = match cur.read_u8() {
            1 => DataEncoding::Lsb,
            2 => DataEncoding::Msb,
            x => return Err(ParseElfError::InvalidDataEncoding(x)),
        };

        let _header_version = cur.read_u8();
        let _os_abi = cur.read_u8();
        let _abi_version = cur.read_u8();
        cur.skip(7); // padding

        let ty = match read_u16(&mut cur, data_encoding) {
            1 => ObjectFileType::RelocatableFile,
            2 => ObjectFileType::ExecutableFile,
            3 => ObjectFileType::SharedObjectFile,
            4 => ObjectFileType::CoreFile,
            x @ 0xFE00..=0xFEFF => ObjectFileType::EnvironmentSpecific(x),
            x @ 0xFF00..=0xFFFF => ObjectFileType::ProcessorSpecific(x),
            x => return Err(ParseElfError::InvalidObjectFileType(x)),
        };

        let machine = MachineArchitecture::try_from(read_u16(&mut cur, data_encoding))
            .map_err(|e| ParseElfError::UnknownMachineArchitecture(e))?;

        let _elf_version = read_u32(&mut cur, data_encoding);

        let entry = read_u64(&mut cur, data_encoding);

        let ph_offset = read_u64(&mut cur, data_encoding);
        let sh_offset = read_u64(&mut cur, data_encoding);

        let flags = read_u32(&mut cur, data_encoding);

        let header_size = read_u16(&mut cur, data_encoding);

        let ph_entry_size = read_u16(&mut cur, data_encoding) as u64;
        let ph_entry_count = read_u16(&mut cur, data_encoding) as u64;
        let sh_entry_size = read_u16(&mut cur, data_encoding) as u64;
        let sh_entry_count = read_u16(&mut cur, data_encoding) as u64;
        let sh_str_index = read_u16(&mut cur, data_encoding) as u64;

        assert!(ph_entry_size == size_of::<Elf64ProgramHeader>() as u64);
        assert!(sh_entry_size == size_of::<Elf64SectionHeader>() as u64);
        assert!(cur.position() == size_of::<Elf64Header>());

        if ph_offset + ph_entry_count * ph_entry_size > buffer.len() as u64 {
            return Err(ParseElfError::BufferTooSmall {
                given: buffer.len() as u64,
                required: ph_offset + ph_entry_count * ph_entry_size,
            });
        }
        let mut program_headers = Vec::new();
        for i in 0..ph_entry_count {
            let start = ph_offset + i * ph_entry_size;
            let program_header = ProgramHeader::try_from(
                &buffer[(start as usize)..((start + ph_entry_size) as usize)],
                data_encoding,
            )?;
            program_headers.push(program_header);
        }

        if sh_offset + sh_entry_count * sh_entry_size > buffer.len() as u64 {
            return Err(ParseElfError::BufferTooSmall {
                given: buffer.len() as u64,
                required: sh_offset + sh_entry_count * sh_entry_size,
            });
        }
        let (string_table, sh_buffer) = {
            let start = sh_offset + sh_str_index * sh_entry_size;
            let header = SectionHeader::try_from(
                &buffer[(start as usize)..((start + sh_entry_size) as usize)],
                data_encoding,
                None,
            )?;
            if header.ty != SectionType::StringTable {
                return Err(ParseElfError::InvalidData);
            }
            let offset = header.offset;
            let size = header.size;

            if offset == sh_offset {
                return Err(ParseElfError::InvalidData);
            } else if offset < sh_offset {
                let (before, after) = buffer.split_at(sh_offset as usize);
                let string_table = &before[offset as usize..(offset as usize + size as usize)];
                let sh_buffer = after;

                (string_table, sh_buffer)
            } else {
                let (before, after) = buffer.split_at(offset as usize);
                let string_table = &after[..(size as usize)];
                let sh_buffer = &before[(sh_offset as usize)..];

                (string_table, sh_buffer)
            }
        };
        let mut section_headers = Vec::new();
        for i in 0..sh_entry_count {
            let start = i * sh_entry_size;
            let section_header = SectionHeader::try_from(
                &sh_buffer[(start as usize)..((start + sh_entry_size) as usize)],
                data_encoding,
                Some(string_table),
            )?;
            section_headers.push(section_header);
        }

        Ok(Self {
            class,
            data_encoding,
            ty,
            machine,
            entry,
            program_headers,
            section_headers,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseElfError {
    MissingMagicNumber,
    InvalidElfClass(u8),
    InvalidDataEncoding(u8),
    InvalidObjectFileType(u16),
    InvalidSegmentType(u32),
    InvalidSectionType(u32),
    UnknownMachineArchitecture(u16),
    BufferTooSmall { given: u64, required: u64 },
    InvalidUtf8,
    InvalidData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    Elf32,
    Elf64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataEncoding {
    Lsb,
    Msb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectFileType {
    RelocatableFile,
    ExecutableFile,
    SharedObjectFile,
    CoreFile,
    EnvironmentSpecific(u16),
    ProcessorSpecific(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum MachineArchitecture {
    None = 0,
    M32 = 1,
    Sparc = 2,
    Intel386 = 3,
    Motorola68K = 4,
    Motorola88K = 5,
    Intel860 = 7,
    Mips = 8,
    PowerPc = 0x14,
    Arm = 0x28,
    SuperH = 0x2A,
    IA64 = 0x32,
    X86_64 = 0x3E,
    Aarch64 = 0xB7,
    RiscV = 0xF3,
}

impl TryFrom<u16> for MachineArchitecture {
    type Error = u16;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(MachineArchitecture::None),
            1 => Ok(MachineArchitecture::M32),
            2 => Ok(MachineArchitecture::Sparc),
            3 => Ok(MachineArchitecture::Intel386),
            4 => Ok(MachineArchitecture::Motorola68K),
            5 => Ok(MachineArchitecture::Motorola88K),
            7 => Ok(MachineArchitecture::Intel860),
            8 => Ok(MachineArchitecture::Mips),
            0x14 => Ok(MachineArchitecture::PowerPc),
            0x28 => Ok(MachineArchitecture::Arm),
            0x2A => Ok(MachineArchitecture::SuperH),
            0x32 => Ok(MachineArchitecture::IA64),
            0x3E => Ok(MachineArchitecture::X86_64),
            0xB7 => Ok(MachineArchitecture::Aarch64),
            0xF3 => Ok(MachineArchitecture::RiscV),
            x => Err(x),
        }
    }
}

type Elf32Header = ElfHeader<u32>;
type Elf64Header = ElfHeader<u64>;

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct ElfHeader<T> {
    ident: ElfHeaderIdent,
    ty: u16,
    machine: u16,
    version: u32,
    entry: T,
    program_header_offset: T,
    section_header_offset: T,
    flags: u32,
    header_size: u16,
    program_header_entry_size: u16,
    program_header_count: u16,
    section_header_entry_size: u16,
    section_header_count: u16,
    shstrndx: u16,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct ElfHeaderIdent {
    magic_number: [u8; 4],
    class: u8,
    data_encoding: u8,
    version: u8,
    pad0: [u8; 9],
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Elf64ProgramHeader {
    pub ty: u32,
    pub flags: u32,
    pub offset: u64,
    pub vaddr: u64,
    pub paddr: u64,
    pub file_size: u64,
    pub mem_size: u64,
    pub alignment: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct ProgramHeader {
    pub ty: SegmentType,
    pub flags: SegmentAttributes,
    pub offset: u64,
    pub vaddr: u64,
    pub paddr: u64,
    pub file_size: u64,
    pub mem_size: u64,
    pub alignment: u64,
}

impl ProgramHeader {
    fn try_from(buffer: &[u8], data_encoding: DataEncoding) -> Result<Self, ParseElfError> {
        assert!(buffer.len() == size_of::<Elf64ProgramHeader>());
        let mut cur = Cursor::new(buffer);

        let ty = SegmentType::try_from(read_u32(&mut cur, data_encoding))?;
        let flags = SegmentAttributes(read_u32(&mut cur, data_encoding));
        let offset = read_u64(&mut cur, data_encoding);
        let vaddr = read_u64(&mut cur, data_encoding);
        let paddr = read_u64(&mut cur, data_encoding);
        let file_size = read_u64(&mut cur, data_encoding);
        let mem_size = read_u64(&mut cur, data_encoding);
        let alignment = read_u64(&mut cur, data_encoding);

        assert!(cur.remaining() == 0);

        Ok(Self {
            ty,
            flags,
            offset,
            vaddr,
            paddr,
            file_size,
            mem_size,
            alignment,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SegmentType {
    Unused = 0,
    Loadable = 1,
    DynamicLinking = 2,
    InterpreterPathName = 3,
    Note = 4,
    SHLIB = 5,
    ProgramHeaderTable = 6,
    EnvironmentSpecific(u32),
    ProcessorSpecific(u32),
}

impl TryFrom<u32> for SegmentType {
    type Error = ParseElfError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(SegmentType::Unused),
            1 => Ok(SegmentType::Loadable),
            2 => Ok(SegmentType::DynamicLinking),
            3 => Ok(SegmentType::InterpreterPathName),
            4 => Ok(SegmentType::Note),
            5 => Ok(SegmentType::SHLIB),
            6 => Ok(SegmentType::ProgramHeaderTable),
            x @ 0x6000_0000..=0x6FFF_FFFF => Ok(SegmentType::EnvironmentSpecific(x)),
            x @ 0x7000_0000..=0x7FFF_FFFF => Ok(SegmentType::ProcessorSpecific(x)),
            x => Err(ParseElfError::InvalidSegmentType(x)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct SegmentAttributes(u32);

impl SegmentAttributes {
    pub fn x(&self) -> bool {
        self.0 & (1 << 0) != 0
    }

    pub fn w(&self) -> bool {
        self.0 & (1 << 1) != 0
    }

    pub fn r(&self) -> bool {
        self.0 & (1 << 2) != 0
    }

    pub fn environment_specific(&self) -> u8 {
        (self.0 >> 16) as u8
    }

    pub fn processor_specific(&self) -> u8 {
        (self.0 >> 24) as u8
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Elf64SectionHeader {
    name: u32,
    ty: u32,
    flags: u64,
    addr: u64,
    offset: u64,
    size: u64,
    link: u32,
    info: u32,
    addralign: u64,
    entry_size: u64,
}

impl Elf64SectionHeader {
    const INDEX_UNDEFINED: u32 = 0;
}

#[derive(Debug, Clone)]
pub struct SectionHeader {
    pub name: String,
    pub ty: SectionType,
    pub flags: SectionAttributes,
    pub vaddr: u64,
    pub offset: u64,
    pub size: u64,
    pub link: u32,
    pub info: u32,
    pub addralign: u64,
    pub entry_size: u64,
}

impl SectionHeader {
    fn try_from(
        buffer: &[u8],
        data_encoding: DataEncoding,
        string_table: Option<&[u8]>,
    ) -> Result<Self, ParseElfError> {
        assert!(buffer.len() == size_of::<Elf64SectionHeader>());

        let mut cur = Cursor::new(buffer);

        let name_offset = read_u32(&mut cur, data_encoding) as usize;
        let name = if let Some(string_table) = string_table {
            let size = string_table[name_offset..]
                .iter()
                .position(|x| *x == 0)
                .expect("unterminated string literal");
            String::from_utf8(string_table[name_offset..(name_offset + size)].to_vec())
                .map_err(|_| ParseElfError::InvalidUtf8)?
        } else {
            format!("offset: {}", name_offset)
        };

        let ty = SectionType::try_from(read_u32(&mut cur, data_encoding))?;
        let flags = SectionAttributes(read_u64(&mut cur, data_encoding));
        let vaddr = read_u64(&mut cur, data_encoding);
        let offset = read_u64(&mut cur, data_encoding);
        let size = read_u64(&mut cur, data_encoding);
        let link = read_u32(&mut cur, data_encoding);
        let info = read_u32(&mut cur, data_encoding);
        let addralign = read_u64(&mut cur, data_encoding);
        let entry_size = read_u64(&mut cur, data_encoding);

        assert!(cur.remaining() == 0);

        Ok(Self {
            name,
            ty,
            flags,
            vaddr,
            offset,
            size,
            link,
            info,
            addralign,
            entry_size,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SectionType {
    Null = 0,
    ProgramBits = 1,
    LinkerSymbolTable = 2,
    StringTable = 3,
    RelaTypeRelocationEntries = 4,
    SymbolHashTable = 5,
    DynamicLinkingTables = 6,
    Note = 7,
    UninitializedSpace = 8,
    RelTypeRelocationEntries = 9,
    SHLIB = 10,
    DynamicLoaderSymbolTable = 11,
    EnvironmentSpecific(u32),
    ProcessorSpecific(u32),
}

impl TryFrom<u32> for SectionType {
    type Error = ParseElfError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(SectionType::Null),
            1 => Ok(SectionType::ProgramBits),
            2 => Ok(SectionType::LinkerSymbolTable),
            3 => Ok(SectionType::StringTable),
            4 => Ok(SectionType::RelaTypeRelocationEntries),
            5 => Ok(SectionType::SymbolHashTable),
            6 => Ok(SectionType::DynamicLinkingTables),
            7 => Ok(SectionType::Note),
            8 => Ok(SectionType::UninitializedSpace),
            9 => Ok(SectionType::RelTypeRelocationEntries),
            10 => Ok(SectionType::SHLIB),
            11 => Ok(SectionType::DynamicLoaderSymbolTable),
            x @ 0x6000_0000..=0x6FFF_FFFF => Ok(SectionType::EnvironmentSpecific(x)),
            x @ 0x7000_0000..=0x7FFF_FFFF => Ok(SectionType::ProcessorSpecific(x)),
            x => Err(ParseElfError::InvalidSectionType(x)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct SectionAttributes(u64);

impl SectionAttributes {
    pub fn writable(&self) -> bool {
        self.0 & (1 << 0) != 0
    }

    pub fn allocated(&self) -> bool {
        self.0 & (1 << 1) != 0
    }

    pub fn executable(&self) -> bool {
        self.0 & (1 << 2) != 0
    }

    pub fn environment_specific(&self) -> u8 {
        (self.0 >> 24) as u8 & 0xF
    }

    pub fn processor_specific(&self) -> u8 {
        (self.0 >> 28) as u8 & 0xF
    }
}
