use core::{fmt::Debug, mem::MaybeUninit, ptr};

use alloc::{borrow::ToOwned, string::String, vec::Vec};

use crate::disk::StorageDevice;

#[derive(Debug)]
pub struct FatFs<'a> {
    storage_device: &'a mut dyn StorageDevice,
    bytes_per_sector: usize,
    sectors_per_cluster: usize,
    sectors_per_fat: usize,
    total_sectors: usize,
    first_fat_sector: usize,
    root_cluster: usize,
    root_directory_entry_count: usize,
    first_data_sector: usize,
}

// TODO: add support for differing bytes_per_sector in fs table and sector size of the actual
// underlying disk or at least assert them being the same for now
impl<'a> FatFs<'a> {
    pub fn wrap(storage_device: &'a mut dyn StorageDevice) -> Result<Self, &'static str> {
        let mut fs = Self {
            storage_device,
            bytes_per_sector: 0,
            sectors_per_cluster: 0,
            sectors_per_fat: 0,
            total_sectors: 0,
            first_fat_sector: 0,
            root_cluster: 0,
            root_directory_entry_count: 0,
            first_data_sector: 0,
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

    fn read_sectors(&mut self, start_sector: usize, count: usize) -> Result<Vec<u8>, &'static str> {
        let mut buffer = alloc::vec![0u8; self.bytes_per_sector * count];
        self.storage_device
            .read(&mut buffer, start_sector as u64, count as u16)?;

        Ok(buffer)
    }

    fn read_cluster(&mut self, cluster: usize) -> Result<Vec<u8>, &'static str> {
        let mut buffer = alloc::vec![0u8; self.bytes_per_sector * self.sectors_per_cluster];
        let sector = self.first_sector_of_cluster(cluster);
        self.storage_device
            .read(&mut buffer, sector as u64, self.sectors_per_cluster as u16)?;

        Ok(buffer)
    }

    fn first_sector_of_cluster(&self, cluster: usize) -> usize {
        ((cluster - 2) * self.sectors_per_cluster) + self.first_data_sector
    }

    pub fn list_directory<S: AsRef<str>>(
        &mut self,
        _path: S,
    ) -> Result<Vec<DirectoryEntry>, &'static str> {
        const BYTES_PER_ENTRY: usize = 32;

        assert!((self.root_directory_entry_count * BYTES_PER_ENTRY) % self.bytes_per_sector == 0);
        let root_sector_count =
            self.root_directory_entry_count * BYTES_PER_ENTRY / self.bytes_per_sector;
        let buffer = self.read_sectors(self.root_cluster, root_sector_count)?;

        let max_i = self
            .root_directory_entry_count
            .min(buffer.len() / BYTES_PER_ENTRY);
        for i in 0..max_i {
            let entry = &buffer[(i * BYTES_PER_ENTRY)..((i + 1) * BYTES_PER_ENTRY)];

            // if the first byte of the entry is equal to 0 then there are no more
            // files/directories in this directory
            if entry[0] == 0 {
                break;
            }

            // if the first byte of the entry is equal to 0xE5 then the entry is unused
            if entry[0] == 0xE5 {
                continue;
            }

            // if the 12th byte is equal to 0x0F then this is a long file name entry
            if entry[11] == 0x0F {
                todo!("long file name entries");
            }

            let start_cluster = ((entry[0x14] as u32) << 24)
                | ((entry[0x15] as u32) << 16)
                | ((entry[0x1A] as u32) << 8)
                | entry[0x1B] as u32;
            // let size = ((entry[0x1C] as u32) << 24)
            //     | ((entry[0x1D] as u32) << 16)
            //     | ((entry[0x1E] as u32) << 8)
            //     | entry[0x1F] as u32;
            let size = u32::from_le_bytes(entry[0x1C..0x20].try_into().unwrap());

            let name = str::from_utf8(&entry[0..8]).unwrap().trim();
            let extension = str::from_utf8(&entry[8..11]).unwrap();
            let full_name = if extension == "   " {
                name.to_owned()
            } else {
                alloc::format!("{}.{}", name, extension)
            };

            crate::println!("> entry {i}: {:?}", entry);
            crate::println!("  > name: {}", full_name);
            crate::println!("  > start cluster: {}", start_cluster);
            crate::println!("  > size: {}", size);
        }

        todo!();
    }
}

#[derive(Debug)]
pub struct DirectoryEntry;

#[derive(Debug, Clone)]
struct FatDirectoryEntry {
    pub name: String,
    pub attributes: u8,
    reserved: u8,
    pub ctime_10ms: u8,
    pub ctime: u16,
    pub cdate: u16,
    pub adate: u16,
    pub start_cluster_h: u16,
    pub mtime: u16,
    pub mdate: u16,
    pub start_cluster_l: u16,
    pub size: u32,
}

impl FatDirectoryEntry {
    fn parse(data: &[u8]) -> Option<Self> {
        if data.len() != 32 {
            return None;
        }

        let name = {
            let name = str::from_utf8(&data[0..8]).ok()?.trim();
            let extension = str::from_utf8(&data[8..11]).ok()?;

            if extension == "   " {
                name.to_owned()
            } else {
                alloc::format!("{}.{}", name, extension)
            }
        };

        let attributes = data[0x0B];
        let ctime_10ms = data[0x0D];
        let ctime = ((data[0x0E] as u16) << 8) | data[0x0F] as u16;
        let cdate = ((data[0x10] as u16) << 8) | data[0x11] as u16;
        let adate = ((data[0x12] as u16) << 8) | data[0x13] as u16;
        let start_cluster_h = ((data[0x14] as u16) << 8) | data[0x15] as u16;
        let mtime = ((data[0x16] as u16) << 8) | data[0x17] as u16;
        let mdate = ((data[0x18] as u16) << 8) | data[0x19] as u16;
        let start_cluster_l = ((data[0x1A] as u16) << 8) | data[0x1B] as u16;
        let size = ((data[0x1C] as u32) << 24)
            | ((data[0x1D] as u32) << 16)
            | ((data[0x1E] as u32) << 8)
            | data[0x1F] as u32;

        Some(Self {
            name,
            attributes,
            reserved: 0,
            ctime_10ms,
            ctime,
            cdate,
            adate,
            start_cluster_h,
            mtime,
            mdate,
            start_cluster_l,
            size,
        })
    }
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
