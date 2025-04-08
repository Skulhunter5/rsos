pub struct Unclassified;

impl Unclassified {
    pub const NON_VGA_COMPATIBLE_UNCLASSIFIED_DEVICE: u8 = 0x0;
    pub const VGA_COMPATIBLE_UNCLASSIFIED_DEVICE: u8 = 0x1;
}

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

pub struct NetworkController;

impl NetworkController {
    pub const ETHERNET_CONTROLLER: u8 = 0x0;
    pub const TOKEN_RING_CONTROLLER: u8 = 0x1;
    pub const FDDI_CONTROLLER: u8 = 0x2;
    pub const ATM_CONTROLLER: u8 = 0x3;
    pub const ISDN_CONTROLLER: u8 = 0x4;
    pub const WORLD_FIP_CONTROLLER: u8 = 0x5;
    pub const PICMG_214_MULTI_COMPUTING_CONTROLLER: u8 = 0x6;
    pub const INFINIBAND_CONTROLLER: u8 = 0x7;
    pub const FABRIC_CONTROLLER: u8 = 0x8;
    pub const OTHER: u8 = 0x80;
}

pub struct DisplayController;

impl DisplayController {
    pub const VGA_COMPATIBLE_CONTROLLER: u8 = 0x0;
    pub const XGA_CONTROLLER: u8 = 0x1;
    pub const CONTROLLER_3D: u8 = 0x2;
    pub const OTHER: u8 = 0x80;
}

pub struct MultimediaController;

impl MultimediaController {
    pub const MULTIMEDIA_VIDEO_CONTROLLER: u8 = 0x0;
    pub const MULTIMEDIA_AUDIO_CONTROLLER: u8 = 0x1;
    pub const COMPUTER_TELEPHONY_DEVICE: u8 = 0x2;
    pub const AUDIO_DEVICE: u8 = 0x3;
    pub const OTHER: u8 = 0x80;
}

pub struct MemoryController;

impl MemoryController {
    pub const RAM_CONTROLLER: u8 = 0x0;
    pub const FLASH_CONTROLLER: u8 = 0x1;
    pub const OTHER: u8 = 0x80;
}

pub struct Bridge;

impl Bridge {
    pub const HOST_BRIDGE: u8 = 0x0;
    pub const ISA_BRIDGE: u8 = 0x1;
    pub const EISA_BRIDGE: u8 = 0x2;
    pub const MCA_BRIDGE: u8 = 0x3;
    pub const PCI_TO_PCI_BRIDGE: u8 = 0x4;
    pub const PCMCIA_BRIDGE: u8 = 0x5;
    pub const NUBUS_BRIDGE: u8 = 0x6;
    pub const CARDBUS_BRIDGE: u8 = 0x7;
    pub const RACEWAY_BRIDGE: u8 = 0x8;
    pub const SEMI_TRANSPARENT_PCI_TO_PCI_BRIDGE: u8 = 0x9;
    pub const INFINIBAND_TO_PCI_HOST_BRIDGE: u8 = 0x0A;
    pub const OTHER: u8 = 0x80;
}
