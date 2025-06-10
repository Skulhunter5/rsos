#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub struct Guid {
    time_low: u32,
    time_mid: u16,
    time_high_and_revision: u16,
    clock_seq_high_and_reserved: u8,
    clock_seq_low: u8,
    node: [u8; 6],
}

impl Guid {
    // EFI_ACPI_TABLE_GUID:
    // [0x8868e871, 0xe4f1, 0x11d3, 0xbc, 0x22, 0x0, 0x80, 0xc7, 0x3c, 0x88, 0x81]
    pub const EFI_ACPI_TABLE_GUID: Guid = Guid::new(
        0x8868e871,
        0xe4f1,
        0x11d3,
        0xbc,
        0x22,
        [0x0, 0x80, 0xc7, 0x3c, 0x88, 0x81],
    );

    pub const fn new(
        time_low: u32,
        time_mid: u16,
        time_high_and_revision: u16,
        clock_seq_high_and_reserved: u8,
        clock_seq_low: u8,
        node: [u8; 6],
    ) -> Self {
        Self {
            time_low,
            time_mid,
            time_high_and_revision,
            clock_seq_high_and_reserved,
            clock_seq_low,
            node,
        }
    }
}
