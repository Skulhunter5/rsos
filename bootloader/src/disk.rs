use alloc::vec::Vec;

#[derive(Debug)]
pub struct Disk<'a> {
    storage_device: &'a mut dyn StorageDevice,
    partitions: Vec<Partition>,
}

impl<'a> Disk<'a> {
    pub fn new(storage_device: &'a mut dyn StorageDevice) -> Result<Self, &'static str> {
        let partitions = Vec::new();
        let mut disk = Self { storage_device, partitions };

        disk.read_partitions()?;

        Ok(disk)
    }

    fn read_partitions(&mut self) -> Result<(), &'static str> {
        let mut buffer = [0u8; 4 * 1024];
        self.storage_device.read(&mut buffer, 0, 1)?;
        if buffer[510..512] != [0x55, 0xAA] {
            return Err("invalid partition table");
        }

        let mut partitions = Vec::new();
        for i in 0..4 {
            let start = 0x1BE + i * 16;
            let end = start + 16;
            let mut bytes = [0u8; 16];
            bytes.copy_from_slice(&buffer[start..end]);
            if let Some(partition) = Partition::try_from_mbr_raw(bytes) {
                partitions.push(partition);
            }
        }

        self.partitions = partitions;

        Ok(())
    }

    pub fn partitions(&self) -> &Vec<Partition> {
        &self.partitions
    }
}

#[derive(Debug)]
pub struct Partition {
    pub ty: u8,
    pub start: u32,
    pub sector_count: u32,
}

impl Partition {
    pub fn try_from_mbr_raw(bytes: [u8; 16]) -> Option<Self> {
        let entry = unsafe { core::mem::transmute::<[u8; 16], MbrPartitionTableEntry>(bytes) };
        if entry.drive_attributes == 0 {
            return None;
        }
        let ty = entry.ty;
        let start = entry.lba_start;
        let sector_count = entry.sector_count;

        Some(Self { ty, start, sector_count })
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct MbrPartitionTableEntry {
    drive_attributes: u8,
    chs_start0: u8,
    chs_start1: u8,
    chs_start2: u8,
    ty: u8,
    chs_last0: u8,
    chs_last1: u8,
    chs_last2: u8,
    lba_start: u32,
    sector_count: u32,
}

pub trait StorageDevice: core::fmt::Debug {
    fn read(&mut self, buffer: &mut [u8], lba: u64, sectors: u16) -> Result<(), &'static str>;
    fn write(&mut self, buffer: &[u8], lba: u64) -> Result<(), &'static str>;
}
