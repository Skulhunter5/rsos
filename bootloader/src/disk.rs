use alloc::vec::Vec;

pub struct Disk;

impl Disk {
    // TODO: add support for GPT
    pub fn read_partitions(
        storage_device: &mut dyn StorageDevice,
    ) -> Result<Vec<Partition>, &'static str> {
        let mut buffer = [0u8; 4 * 1024];
        storage_device.read(&mut buffer, 0, 1)?;
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

        Ok(partitions)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Partition {
    pub ty: u8,
    pub start: u64,
    pub sector_count: u64,
}

impl Partition {
    pub fn try_from_mbr_raw(bytes: [u8; 16]) -> Option<Self> {
        let entry = unsafe { core::mem::transmute::<[u8; 16], MbrPartitionTableEntry>(bytes) };
        if entry.drive_attributes == 0 {
            return None;
        }
        let ty = entry.ty;
        let start = entry.lba_start as u64;
        let sector_count = entry.sector_count as u64;

        Some(Self {
            ty,
            start,
            sector_count,
        })
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

#[derive(Debug)]
pub struct PartitionDevice<'a> {
    storage_device: &'a mut dyn StorageDevice,
    partition: Partition,
}

impl<'a> PartitionDevice<'a> {
    pub fn new(storage_device: &'a mut dyn StorageDevice, partition: Partition) -> Self {
        Self {
            storage_device,
            partition,
        }
    }
}

impl StorageDevice for PartitionDevice<'_> {
    fn read(&mut self, buffer: &mut [u8], lba: u64, sectors: u16) -> Result<(), &'static str> {
        if (lba + sectors as u64) > self.partition.sector_count {
            return Err("error: sector out of range for partition");
        }
        self.storage_device
            .read(buffer, self.partition.start, sectors)
    }

    fn write(&mut self, _buffer: &[u8], _lba: u64) -> Result<(), &'static str> {
        todo!();
    }
}
