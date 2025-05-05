use super::SdtHeader;

#[repr(C)]
pub struct Rsdt {
    pub header: SdtHeader,
    pub tables: [u32],
}

#[repr(C)]
pub struct Xsdt {
    pub header: SdtHeader,
    pub tables: [u64],
}
