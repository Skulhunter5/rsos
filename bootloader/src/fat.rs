use core::{fmt::Debug, mem::MaybeUninit, ptr};

use alloc::vec::Vec;

use crate::disk::StorageDevice;

#[derive(Debug)]
pub struct FatFs<'a> {
    storage_device: &'a mut dyn StorageDevice,
    bpb: BiosParameterBlock,
    bytes_per_sector: usize,
    sectors_per_cluster: usize,
    sectors_per_fat: usize,
    total_sectors: usize,
    first_fat_sector: usize,
    root_cluster: usize,
    first_data_sector: usize,
}

impl<'a> FatFs<'a> {
    pub fn wrap(storage_device: &'a mut dyn StorageDevice) -> Result<Self, &'static str> {
        let fs = Self::init(storage_device)?;

        Ok(fs)
    }

    fn init(storage_device: &'a mut dyn StorageDevice) -> Result<Self, &'static str> {
        let bpb = Self::read_bpb(storage_device)?;
        crate::println!("bpb: {:?}", &bpb);

        let bytes_per_sector = bpb.bytes_per_sector as usize;
        let sectors_per_cluster = bpb.sectors_per_cluster as usize;
        let reserved_sectors = bpb.reserved_sectors as usize;
        let fat_count = bpb.fat_count as usize;
        let sectors_per_fat = bpb.sectors_per_fat as usize;
        let root_directory_entry_count = bpb.root_directory_entry_count as usize;

        let root_dir_sectors =
            ((root_directory_entry_count * 32) + (bytes_per_sector - 1)) / bytes_per_sector;

        if bytes_per_sector == 0 {
            return Err("unsupported FAT type");
        }

        let total_sectors = if bpb.sector_count != 0 {
            bpb.sector_count as usize
        } else {
            bpb.large_sector_count as usize
        };
        let first_fat_sector = bpb.reserved_sectors as usize;

        let data_sectors =
            total_sectors - (reserved_sectors + (fat_count * sectors_per_fat) + root_dir_sectors);
        let total_clusters = data_sectors / sectors_per_cluster;
        if total_clusters < 65525 {
            return Err("unsupported FAT type");
        }

        let header = Self::read_header(storage_device)?;
        // crate::println!("header: {:?}", &header);
        let root_cluster = header.root_cluster as usize;

        let first_data_sector = reserved_sectors + (fat_count * sectors_per_fat) + root_dir_sectors;

        crate::println!("root_cluster: {}", root_cluster);
        crate::println!("root_dir_sectors: {}", root_dir_sectors);
        crate::println!("first_data_sector: {}", first_data_sector);
        crate::println!("test: {}", core::mem::offset_of!(Fat32Header, root_cluster));

        Ok(Self {
            storage_device,
            bpb,
            bytes_per_sector,
            sectors_per_cluster,
            sectors_per_fat,
            total_sectors,
            first_fat_sector,
            root_cluster,
            first_data_sector,
        })
    }

    fn read_bpb(
        storage_device: &mut dyn StorageDevice,
    ) -> Result<BiosParameterBlock, &'static str> {
        let mut buffer = [0u8; 4 * 1024];
        storage_device.read(&mut buffer, 0, 1)?;
        let mut bpb = MaybeUninit::<BiosParameterBlock>::uninit();

        unsafe {
            ptr::copy_nonoverlapping(
                buffer.as_ptr(),
                bpb.as_mut_ptr() as *mut u8,
                size_of::<BiosParameterBlock>(),
            );

            return Ok(bpb.assume_init());
        }
    }

    fn read_header(storage_device: &mut dyn StorageDevice) -> Result<Fat32Header, &'static str> {
        let mut buffer = [0u8; 4 * 1024];
        storage_device.read(&mut buffer, 0, 1)?;
        let mut header = MaybeUninit::<Fat32Header>::uninit();

        unsafe {
            ptr::copy_nonoverlapping(
                buffer.as_ptr(),
                header.as_mut_ptr() as *mut u8,
                size_of::<Fat32Header>(),
            );

            return Ok(header.assume_init());
        }
    }

    fn read_from_disk<T: Copy + Sized>(&mut self) -> Result<T, &'static str> {
        assert!(size_of::<T>() <= self.bytes_per_sector);

        // let mut buffer = Vec::with_capacity(self.bps);
        let mut buffer = [0u8; 4 * 1024];
        self.storage_device.read(&mut buffer, 0, 1)?;
        let mut item = MaybeUninit::<T>::uninit();

        unsafe {
            ptr::copy_nonoverlapping(
                buffer.as_ptr(),
                item.as_mut_ptr() as *mut u8,
                size_of::<T>(),
            );

            return Ok(item.assume_init());
        }
    }

    fn read_sector(&mut self, sector: usize) -> Result<Vec<u8>, &'static str> {
        let mut buffer = alloc::vec![0u8; self.bytes_per_sector];
        self.storage_device.read(&mut buffer, sector as u64, 1)?;

        Ok(buffer)
    }

    fn read_cluster(&mut self, cluster: usize) -> Result<Vec<u8>, &'static str> {
        let mut buffer = alloc::vec![0u8; self.bytes_per_sector * self.sectors_per_cluster];
        let sector = self.first_sector_of_cluster(cluster);
        crate::println!("cluster-sector: {}", sector);
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
        let buffer = self.read_cluster(self.root_cluster)?;
        crate::println!("root_cluster: {:?}", &buffer);

        todo!();
    }
}

#[derive(Debug)]
pub struct DirectoryEntry;

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
