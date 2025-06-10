#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct SdtHeader {
    pub signature: [u8; 4],
    pub length: u32,
    pub revision: u8,
    pub checksum: u8,
    pub oem_id: [u8; 6],
    pub oem_table_id: [u8; 8],
    pub oem_revision: u32,
    pub creator_id: u32,
    pub creator_revision: u32,
}

impl SdtHeader {
    pub unsafe fn validate(&self) -> bool {
        let ptr = self as *const SdtHeader;
        let ptr = ptr as *const u8;
        let size = self.length as usize;
        let sum = (0..size)
            .map(|i| unsafe { *ptr.add(i) } as usize)
            .sum::<usize>();

        return sum & 0xFF == 0;
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct Mcfg {
    pub header: SdtHeader,
    pub reserved: u64,
    pub base_address_allocations: [ConfigurationSpaceBaseAddressAllocation],
}

#[derive(Debug)]
#[repr(C)]
pub struct ConfigurationSpaceBaseAddressAllocation {
    pub base_address: u64,
    pub segment_group: u16,
    pub start_bus_number: u8,
    pub end_bus_number: u8,
    pub reserved: u32,
}
