use core::{fmt::Debug, mem::MaybeUninit, ptr};

use alloc::vec::Vec;

use crate::disk::StorageDevice;

#[derive(Debug)]
pub struct FatFs<'a> {
    storage_device: &'a mut dyn StorageDevice,
    bps: usize,
    spc: usize,
}

impl<'a> FatFs<'a> {
    pub fn wrap(storage_device: &'a mut dyn StorageDevice) -> Result<Self, &'static str> {
        let mut fs = Self::init(storage_device)?;
        let _header = fs.read_from_disk::<Fat32Header>()?;
        let _bpb = fs.read_from_disk::<BiosParameterBlock>()?;

        Ok(fs)
    }

    fn init(storage_device: &'a mut dyn StorageDevice) -> Result<Self, &'static str> {
        let bpb = Self::read_bpb(storage_device)?;
        let bps = unsafe { (&raw const bpb.bytes_per_sector).read_unaligned() } as usize;
        let spc = unsafe { (&raw const bpb.sectors_per_cluster).read_unaligned() } as usize;
        crate::println!("bytes_per_sector: {}", bps);
        crate::println!("sectors_per_cluster: {}", spc);
        crate::println!("bpb: {:?}", &bpb);

        Ok(Self { storage_device, bps, spc })
    }

    fn read_bpb(storage_device: &mut dyn StorageDevice) -> Result<BiosParameterBlock, &'static str> {
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

    fn read_from_disk<T: Copy + Sized>(&mut self) -> Result<T, &'static str> {
        assert!(size_of::<T>() <= self.bps);

        // let mut buffer = Vec::with_capacity(self.bps);
        let mut buffer = [0u8; 4 * 1024];
        self.storage_device.read(&mut buffer, 0, 1)?;
        let mut bpb = MaybeUninit::<T>::uninit();
        
        unsafe {
            ptr::copy_nonoverlapping(
                buffer.as_ptr(),
                bpb.as_mut_ptr() as *mut u8,
                size_of::<T>(),
            );

            return Ok(bpb.assume_init());
        }
    }

    pub fn list_directory<S: AsRef<str>>(&mut self, _path: S) -> Result<Vec<DirectoryEntry>, &'static str> {
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

#[derive(Clone, Copy)]
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

impl Debug for Fat32Header {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        todo!()
    }
}
