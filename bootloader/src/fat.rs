use core::{fmt::Debug, mem::MaybeUninit, ptr};

use alloc::{borrow::ToOwned, boxed::Box, string::String, vec::Vec};

use crate::disk::StorageDevice;

#[derive(Debug)]
pub struct FatFs<'a> {
    storage_device: &'a mut dyn StorageDevice,
    bytes_per_sector: usize,
    sectors_per_cluster: usize,
    total_sectors: usize,
    first_fat_sector: usize,
    sectors_per_fat: usize,
    root_cluster: usize,
    root_directory_entry_count: usize,
    first_data_sector: usize,
    fat: Option<Fat>,
}

// TODO: add support for differing bytes_per_sector in fs table and sector size of the actual
// underlying disk or at least assert them being the same for now
impl<'a> FatFs<'a> {
    pub fn wrap(storage_device: &'a mut dyn StorageDevice) -> Result<Self, &'static str> {
        let mut fs = Self {
            storage_device,
            bytes_per_sector: 0,
            sectors_per_cluster: 0,
            total_sectors: 0,
            first_fat_sector: 0,
            sectors_per_fat: 0,
            root_cluster: 0,
            root_directory_entry_count: 0,
            first_data_sector: 0,
            fat: None,
        };
        fs.init()?;

        Ok(fs)
    }

    fn init(&mut self) -> Result<(), &'static str> {
        let header = Self::read_header(self.storage_device)?;

        let bpb = match &header {
            Header::Legacy(header) => &header.bpb,
            Header::Fat32(header) => &header.bpb,
        };
        if bpb.bytes_per_sector == 0 {
            return Err("unsupported FAT type: exFAT");
        }

        self.bytes_per_sector = bpb.bytes_per_sector as usize;
        if self.bytes_per_sector != self.storage_device.sector_size()? {
            todo!("FAT sector size != disk sector size");
        }
        self.sectors_per_cluster = bpb.sectors_per_cluster as usize;
        self.sectors_per_fat = bpb.sectors_per_fat as usize;

        let reserved_sectors = bpb.reserved_sectors as usize;
        let fat_count = bpb.fat_count as usize;

        self.root_directory_entry_count = bpb.root_directory_entry_count as usize;
        let root_dir_sectors = match &header {
            Header::Legacy(_) => {
                ((self.root_directory_entry_count * 32) + (self.bytes_per_sector - 1))
                    / self.bytes_per_sector
            }
            Header::Fat32(_) => 0,
        };

        self.total_sectors = if bpb.sector_count != 0 {
            bpb.sector_count as usize
        } else {
            bpb.large_sector_count as usize
        };
        self.first_fat_sector = bpb.reserved_sectors as usize;

        // let data_sectors = self.total_sectors
        //     - (reserved_sectors + (fat_count * self.sectors_per_fat) + root_dir_sectors);
        // let total_clusters = data_sectors / self.sectors_per_cluster;

        self.first_data_sector =
            reserved_sectors + (fat_count * self.sectors_per_fat) + root_dir_sectors;

        self.root_cluster = match &header {
            Header::Legacy(_) => {
                // let root_sector = self.first_data_sector - root_dir_sectors;
                // assert!(root_sector % self.sectors_per_cluster == 0);
                // let root_cluster = root_sector / self.sectors_per_cluster;
                //
                // root_cluster
                self.first_data_sector - root_dir_sectors
            }
            Header::Fat32(header) => header.root_cluster as usize,
        };

        if let Header::Fat32(_) = header {
            todo!("fat32 support");
            // TODO: ensure that everything is fat32-compatible and add necessary things if not
            // for example, in FAT32, the root directories seem to be stored just like any other
            // data, adjust this to be the case instead of reading from the start as in FAT 12/16
        }

        Ok(())
    }

    fn read_header(storage_device: &mut dyn StorageDevice) -> Result<Header, &'static str> {
        let mut buffer = [0u8; 4 * 1024];
        storage_device.read(&mut buffer, 0, 1)?;

        assert!(buffer.len() >= size_of::<BiosParameterBlock>());
        assert!(buffer.len() >= size_of::<LegacyHeader>());
        assert!(buffer.len() >= size_of::<Fat32Header>());

        fn copy<T: Copy>(buffer: &[u8]) -> T {
            let mut bpb = MaybeUninit::<T>::uninit();
            unsafe {
                ptr::copy_nonoverlapping(
                    buffer.as_ptr(),
                    bpb.as_mut_ptr() as *mut u8,
                    size_of::<T>(),
                );

                bpb.assume_init()
            }
        }

        let bpb = copy::<BiosParameterBlock>(&buffer);

        let total_sectors = if bpb.sector_count != 0 {
            bpb.sector_count as usize
        } else {
            bpb.large_sector_count as usize
        };
        let total_clusters = total_sectors / bpb.sectors_per_cluster as usize;

        let header = match total_clusters {
            ..4085 | ..65525 => Header::Legacy(copy::<LegacyHeader>(&buffer)),
            _ => Header::Fat32(copy::<Fat32Header>(&buffer)),
        };

        Ok(header)
    }

    fn read_sector(&mut self, sector: usize) -> Result<Vec<u8>, &'static str> {
        let mut buffer = alloc::vec![0u8; self.bytes_per_sector];
        self.storage_device.read(&mut buffer, sector as u64, 1)?;

        Ok(buffer)
    }

    fn read_sectors(
        &mut self,
        start_sector: usize,
        count: usize,
    ) -> Result<Box<[u8]>, &'static str> {
        let mut buffer = alloc::vec![0u8; self.bytes_per_sector * count];
        self.storage_device
            .read(&mut buffer, start_sector as u64, count as u16)?;

        Ok(buffer.into_boxed_slice())
    }

    fn read_cluster(&mut self, cluster: usize) -> Result<Box<[u8]>, &'static str> {
        let mut buffer = alloc::vec![0u8; self.bytes_per_sector * self.sectors_per_cluster];
        let sector = self.first_sector_of_cluster(cluster);
        self.storage_device
            .read(&mut buffer, sector as u64, self.sectors_per_cluster as u16)?;

        Ok(buffer.into_boxed_slice())
    }

    fn read_cluster_into(&mut self, cluster: usize, buffer: &mut [u8]) -> Result<(), &'static str> {
        if buffer.len() < self.sectors_per_cluster * self.bytes_per_sector {
            return Err("buffer too small");
        }

        let sector = self.first_sector_of_cluster(cluster);
        self.storage_device
            .read(buffer, sector as u64, self.sectors_per_cluster as u16)?;

        Ok(())
    }

    fn first_sector_of_cluster(&self, cluster: usize) -> usize {
        ((cluster - 2) * self.sectors_per_cluster) + self.first_data_sector
    }

    fn read_cluster_chain(&mut self, first_cluster: usize) -> Result<Vec<u8>, &'static str> {
        let fat = if let Some(fat) = &self.fat {
            fat
        } else {
            let fat = self.read_fat()?;
            self.fat = Some(fat);
            self.fat.as_ref().unwrap()
        };
        let cluster_chain = fat.get_cluster_chain(first_cluster)?;

        let cluster_size = self.sectors_per_cluster * self.bytes_per_sector;
        let mut buffer = alloc::vec![0u8; cluster_chain.len() * cluster_size];
        for i in 0..cluster_chain.len() {
            let start = i * self.sectors_per_cluster * self.bytes_per_sector;
            let end = start + cluster_size;
            self.read_cluster_into(cluster_chain[i], &mut buffer[start..end])?;
        }

        Ok(buffer)
    }

    fn read_fat(&mut self) -> Result<Fat, &'static str> {
        let data = self.read_sectors(self.first_fat_sector, self.sectors_per_fat)?;
        return Ok(Fat { data, ty: FatType::Fat16 });
    }

    fn list_directory_raw(&mut self, dir: Option<FatDirectoryEntry>) -> Result<Box<[FatDirectoryEntry]>, &'static str> {
        let mut entries = Vec::new();

        let buffer = if let Some(entry) = dir {
            if !entry.is_directory() {
                return Err("list_directory_raw: not a directory");
            }
            let buffer = self.read_cluster_chain(entry.start_cluster as usize)?;
            buffer.into_boxed_slice()
        } else { // list root directory
            assert!((self.root_directory_entry_count * FatDirectoryEntry::BYTES_PER_ENTRY) % self.bytes_per_sector == 0);
            let root_sector_count =
                self.root_directory_entry_count * FatDirectoryEntry::BYTES_PER_ENTRY / self.bytes_per_sector;
            let buffer = self.read_sectors(self.root_cluster, root_sector_count)?;
            assert!(buffer.len() == self.root_directory_entry_count * FatDirectoryEntry::BYTES_PER_ENTRY);

            buffer
        };
        let mut cur = Cursor::new(&buffer);

        while cur.remaining() >= FatDirectoryEntry::BYTES_PER_ENTRY {
            match FatDirectoryEntry::read_from(&mut cur) {
                (Some(entry), done) => {
                    assert!(!done);
                    entries.push(entry);
                }
                (None, done) => {
                    if done {
                        break;
                    } else {
                        cur.skip(FatDirectoryEntry::BYTES_PER_ENTRY);
                    }
                }
            }
        }

        Ok(entries.into_boxed_slice())
    }

    pub fn list_directory<S: AsRef<str>>(
        &mut self,
        path: S,
    ) -> Result<Box<[FatDirectoryEntry]>, &'static str> {
        if !path.as_ref().starts_with('/') {
            return Err("invalid path: must start with /");
        }

        let tokens = path.as_ref().split('/').collect::<Vec<&str>>();

        let mut current_dir = self.list_directory_raw(None)?;
        for token in &tokens {
            if token.is_empty() {
                continue;
            }
            crate::println!("Token: {}", token);
            let entry = current_dir.iter().find(|entry| entry.name.to_uppercase() == token.to_uppercase());
            if let Some(entry) = entry {
                current_dir = self.list_directory_raw(Some(entry.clone()))?;
            } else {
                return Err("no such file or directory");
            }
        }

        Ok(current_dir)
    }

    fn read_file_raw(&mut self, entry: FatDirectoryEntry) -> Result<Box<[u8]>, &'static str> {
        if !entry.is_file() {
            return Err("not a file");
        }

        let first_cluster = entry.start_cluster as usize;
        let mut file_content = self.read_cluster_chain(first_cluster)?;
        let file_size = entry.size as usize;
        if file_size > file_content.len() {
            return Err("invalid file size");
        }
        file_content.resize(file_size, 0);

        Ok(file_content.into_boxed_slice())
    }

    pub fn read_file<S: AsRef<str>>(&mut self, path: S) -> Result<Box<[u8]>, &'static str> {
        if !path.as_ref().starts_with('/') {
            return Err("invalid path: must start with /");
        }

        let tokens = path.as_ref().split('/').collect::<Vec<&str>>();
        let index = tokens.iter().rev().position(|token| !token.is_empty());
        if let Some(index) = index {
            let index = tokens.len() - 1 - index;

            let mut current_dir = self.list_directory_raw(None)?;
            for token in &tokens[..index] {
                if token.is_empty() {
                    continue;
                }
                crate::println!("Token: {}", token);
                let entry = current_dir.iter().find(|entry| entry.name.to_uppercase() == token.to_uppercase());
                if let Some(entry) = entry {
                    current_dir = self.list_directory_raw(Some(entry.clone()))?;
                } else {
                    return Err("no such file or directory");
                }
            }

            let file_token = tokens[index];
            assert!(!file_token.is_empty());
            let entry = current_dir.iter().find(|entry| entry.name.to_uppercase() == file_token.to_uppercase());
            if let Some(entry) = entry {
                if !entry.is_file() {
                    return Err("not a file");
                }
                return self.read_file_raw(entry.clone());
            } else {
                return Err("no such file or directory");
            }
        } else {
            return Err("no such file or directory");
        }
    }
}

#[derive(Debug)]
struct Fat {
    data: Box<[u8]>,
    ty: FatType,
}

impl Fat {
    fn get_cluster_chain(&self, first_cluster: usize) -> Result<Box<[usize]>, &'static str> {
        match self.ty {
            FatType::Fat12 => todo!(),
            FatType::Fat16 => {
                let mut clusters = Vec::new();
                let mut current = first_cluster;
                loop {
                    clusters.push(current);

                    let index = current * 2;
                    let next = u16::from_le_bytes(self.data[index..(index + 2)].try_into().unwrap())
                        as usize;
                    current = next;

                    match next {
                        0 => return Err("free cluster in cluster chain"),
                        0xFFF7 => return Err("defect cluster in cluster chain"),
                        0x1 => return Err("1 in cluster chain (reserved)"),
                        0xFFF8.. => break, // last cluster in chain
                        0x0002..=0xFFF6 => continue,
                    }
                }

                Ok(clusters.into_boxed_slice())
            }
            FatType::Fat32 => todo!(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FatType {
    Fat12,
    Fat16,
    Fat32,
}

#[derive(Debug, Clone)]
pub struct FatDirectoryEntry {
    pub name: String,
    pub long_name: Option<String>,
    pub attributes: u8,
    pub ctime_10ms: u8,
    pub ctime: u16,
    pub cdate: u16,
    pub adate: u16,
    pub mtime: u16,
    pub mdate: u16,
    pub start_cluster: u32,
    pub size: u32,
}

impl FatDirectoryEntry {
    const BYTES_PER_ENTRY: usize = 32;

    fn read_from(buffer: &mut Cursor) -> (Option<Self>, bool) {
        let start_position = buffer.position();
        assert!(buffer.remaining() >= Self::BYTES_PER_ENTRY);

        // TODO: implement LFN or decide not to
        let long_name = None;
        while buffer.remaining() >= Self::BYTES_PER_ENTRY && buffer.peek_u8(11) == 0x0F {
            // crate::println!("LFN bytes: {:?}", buffer.read_bytes(32));
            // todo!("long file name entries");
            buffer.skip(Self::BYTES_PER_ENTRY);
        }

        if buffer.remaining() < Self::BYTES_PER_ENTRY {
            return (None, false);
        }

        // if the first byte of the entry is equal to 0 then there are no more
        // files/directories in this directory
        if buffer.peek_u8(0) == 0 {
            return (None, true);
        }

        let name = {
            let buffer = buffer.read_bytes(11);

            // let name = str::from_utf8(&buffer[0..8]).ok()?.trim();
            let name = match str::from_utf8(&buffer[0..8]) {
                Ok(s) => s.trim(),
                Err(_) => return (None, false),
            };
            // let extension = str::from_utf8(&buffer[8..11]).ok()?.trim();
            let extension = match str::from_utf8(&buffer[8..11]) {
                Ok(s) => s.trim(),
                Err(_) => return (None, false),
            };

            if extension.is_empty() {
                name.to_owned()
            } else {
                alloc::format!("{}.{}", name, extension)
            }
        };

        let attributes = buffer.read_u8();

        let _reserved = buffer.read_u8();
        let ctime_10ms = buffer.read_u8();
        let ctime = buffer.read_u16_le();
        let cdate = buffer.read_u16_le();
        let adate = buffer.read_u16_le();
        let start_cluster_h = buffer.read_u16_le();
        let mtime = buffer.read_u16_le();
        let mdate = buffer.read_u16_le();
        let start_cluster_l = buffer.read_u16_le();
        let size = buffer.read_u32_le();

        let start_cluster = ((start_cluster_h as u32) << 16) | start_cluster_l as u32;

        let end_position = buffer.position();
        let total_read = end_position - start_position;
        assert!(total_read % Self::BYTES_PER_ENTRY == 0);

        (
            Some(Self {
                name,
                long_name,
                attributes,
                ctime_10ms,
                ctime,
                cdate,
                adate,
                mtime,
                mdate,
                start_cluster,
                size,
            }),
            false,
        )
    }

    fn is_volume_id(&self) -> bool {
        self.attributes & 0x08 != 0
    }

    fn is_directory(&self) -> bool {
        self.attributes & 0x10 != 0
    }

    // TODO: confirm whether this is the correct check
    fn is_file(&self) -> bool {
        !(self.is_volume_id() || self.is_directory())
    }
}

#[derive(Debug)]
pub struct Cursor<'a> {
    buffer: &'a [u8],
    pos: usize,
}

#[allow(unused)]
impl<'a> Cursor<'a> {
    pub fn new<B: AsRef<[u8]> + ?Sized>(buffer: &'a B) -> Self {
        let buffer = buffer.as_ref();
        Self { buffer, pos: 0 }
    }
}

#[allow(unused)]
impl Cursor<'_> {
    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn remaining(&self) -> usize {
        self.buffer.len() - self.pos
    }

    pub fn skip(&mut self, n: usize) {
        if self.remaining() < n {
            panic!("out of bounds: not enough bytes remaining");
        }
        self.pos += n;
    }

    pub fn peek_u8(&self, n: usize) -> u8 {
        if self.remaining() < n + 1 {
            panic!("out of bounds: not enough bytes remaining");
        }
        return self.buffer[self.pos + n];
    }

    pub fn read_u8(&mut self) -> u8 {
        if self.remaining() < 1 {
            panic!("out of bounds: not enough bytes remaining");
        }
        let x = self.buffer[self.pos];
        self.pos += 1;
        return x;
    }

    fn get_bytes<const N: usize>(&mut self) -> [u8; N] {
        if self.remaining() < N {
            panic!("out of bounds: not enough bytes remaining in buffer");
        }
        let x = &self.buffer[self.pos..(self.pos + N)];
        self.pos += N;
        return x.try_into().unwrap();
    }

    pub fn read(&mut self, buffer: &mut [u8]) {
        if self.remaining() < buffer.len() {
            panic!("out of bounds: not enough bytes remaining in buffer");
        }
        buffer.copy_from_slice(&self.buffer[self.pos..(self.pos + buffer.len())]);
        self.pos += buffer.len();
    }

    pub fn read_bytes(&mut self, count: usize) -> &[u8] {
        if self.remaining() < count {
            panic!("out of bounds: not enough bytes remaining in buffer");
        }
        let res = &self.buffer[self.pos..(self.pos + count)];
        self.pos += count;
        return res;
    }
}

// #![feature(macro_metavar_expr_concat)]

macro_rules! cursor_impl_read {
    { $($name_ne:ident, $name_le:ident, $name_be:ident, $t:ty);* $(;)? } => {
        #[allow(unused)]
        impl Cursor<'_> {
            $(
                pub fn $name_ne(&mut self) -> $t {
                    <$t>::from_ne_bytes(self.get_bytes())
                }

                pub fn $name_le(&mut self) -> $t {
                    <$t>::from_ne_bytes(self.get_bytes())
                }

                pub fn $name_be(&mut self) -> $t {
                    <$t>::from_ne_bytes(self.get_bytes())
                }
            )*
        }
    }
}

cursor_impl_read! {
    read_u16, read_u16_le, read_u16_be, u16;
    read_i16, read_i16_le, read_i16_be, i16;
    read_u32, read_u32_le, read_u32_be, u32;
    read_i32, read_i32_le, read_i32_be, i32;
    read_u64, read_u64_le, read_u64_be, u64;
    read_i64, read_i64_le, read_i64_be, i64;
    read_u128, read_u128_le, read_u128_be, u128;
    read_i128, read_i128_le, read_i128_be, i128;
    read_usize, read_usize_le, read_usize_be, usize;
    read_isize, read_isize_le, read_isize_be, isize;
}

#[derive(Debug, Clone, Copy)]
enum Header {
    Legacy(LegacyHeader),
    Fat32(Fat32Header),
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct BiosParameterBlock {
    jump: [u8; 3],
    oem_identifier: [u8; 8],
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    reserved_sectors: u16,
    fat_count: u8,
    root_directory_entry_count: u16,
    sector_count: u16,
    media_descriptor_type: u8,
    sectors_per_fat: u16,
    sectors_per_track: u16,
    head_count: u16,
    hidden_sector_count: u32,
    large_sector_count: u32,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct LegacyHeader {
    bpb: BiosParameterBlock,
    drive_number: u8,
    flags_reserved: u8,
    signature: u8,
    volume_id_serial_number: u32,
    volume_label: [u8; 11],
    system_identifier: [u8; 8],
    boot_code: [u8; 448],
    bootable_partition_signature: [u8; 2],
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct Fat32Header {
    bpb: BiosParameterBlock,
    sectors_per_fat: u32,
    flags: u16,
    fat_version: u16,
    root_cluster: u32,
    fsinfo_cluster: u16,
    backup_boot_sector_cluster: u16,
    reserved: [u8; 12],
    drive_number: u8,
    flags_reserved: u8,
    signature: u8,
    volume_id_serial_number: u32,
    volume_label: [u8; 11],
    system_identifier: [u8; 8],
    boot_code: [u8; 420],
    bootable_partition_signature: [u8; 2],
}

#[repr(C, packed)]
struct FsInfo {
    lead_signature: u32,
    reserved: [u8; 480],
    another_signature: u32,
    last_free_cluster_count: u32,
    next_available_cluster_look: u32,
    reserved2: [u8; 12],
    trail_signature: u32,
}
