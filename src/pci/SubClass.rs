pub struct MassStorageController;

impl MassStorageController {
    pub const SCSI_BUS_CONTROLLER: u8 = 0x0;
    pub const IDE_CONTROLLER: u8 = 0x1;
    pub const FLOPPY_DISK_CONTROLLER: u8 = 0x2;
    pub const IPI_BUS_CONTROLLER: u8 = 0x3;
    pub const RAID_CONTROLLER: u8 = 0x4;
    pub const ATA_CONTROLLER: u8 = 0x5;
    pub const SERIAL_ATA_CONTROLLER: u8 = 0x6;
    pub const SERIAL_ATTACHED_SCSI_CONTROLLER: u8 = 0x7;
    pub const NON_VOLATILE_MEMORY_CONTROLLER: u8 = 0x8;
    pub const OTHER: u8 = 0x80;
}
