use core::fmt::Debug;

#[derive(Debug)]
#[repr(C, packed)]
pub struct BiosParameterBlock {
    pub jump: [u8; 3],
    pub oem_identifier: [u8; 8],
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub fat_count: u8,
    pub root_directory_entry_count: u16,
    pub sector_count: u16,
    pub media_descriptor_type: u8,
    pub sectors_per_fat: u16,
    pub sectors_per_track: u16,
    pub head_count: u16,
    pub hidden_sector_count: u32,
    pub large_sector_count: u32,
}

#[repr(C, packed)]
pub struct Fat32Header {
    bpb: BiosParameterBlock,
    sectors_per_fat: u32,
    flags: u16,
    // TODO: add remaining fields
}

impl Debug for Fat32Header {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        todo!()
    }
}
