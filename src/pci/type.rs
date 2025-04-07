use super::SubClass;

pub struct ClassCode;

#[derive(Debug, Clone, Copy)]
pub enum DeviceType {
    Unclassified { vga_compatible: bool },
    MassStorageController(MassStorageControllerType),
    NetworkController,
    DisplayController,
    MultimediaController,
    MemoryController,
    Bridge,
    SimpleCommunicationController,
    BaseSystemPeripheral,
    InputDeviceController,
    DockingStation,
    Processor,
    SerialBusController,
    WirelessController,
    IntelligentController,
    SatelliteCommunicationController,
    EncryptionController,
    SignalProcessingController,
    ProcessingAccelerator,
    NonEssentialInstrumentation,
    CoProcessor,
    UnassignedClass,
}

impl DeviceType {
    pub fn try_from(class_code: u8, subclass_code: u8, prog_if: u8) -> Result<Self, (u8, u8, u8)> {
        match class_code {
            ClassCode::UNCLASSIFIED => match subclass_code {
                SubclassUnclassified::NON_VGA_COMPATIBLE_UNCLASSIFIED_DEVICE => {
                    Ok(Self::Unclassified {
                        vga_compatible: false,
                    })
                }
                SubclassUnclassified::VGA_COMPATIBLE_UNCLASSIFIED_DEVICE => {
                    Ok(Self::Unclassified {
                        vga_compatible: true,
                    })
                }
                _ => Err((class_code, subclass_code, prog_if)),
            },
            ClassCode::MASS_STORAGE_CONTROLLER => match subclass_code {
                SubClass::MassStorageController::SCSI_BUS_CONTROLLER => Ok(
                    Self::MassStorageController(MassStorageControllerType::ScsiBusController),
                ),
                SubClass::MassStorageController::IDE_CONTROLLER => {
                    if let Some((
                        supports_isa_mode,
                        supports_pci_mode,
                        supports_bus_mastering,
                        mode,
                    )) = match prog_if {
                        0x0 => Some((true, false, false, IdeControllerMode::IsaCompatibilityMode)),
                        0x5 => Some((false, true, false, IdeControllerMode::PciNativeMode)),
                        0xA => Some((true, true, false, IdeControllerMode::IsaCompatibilityMode)),
                        0xF => Some((true, true, false, IdeControllerMode::PciNativeMode)),
                        0x80 => Some((true, false, true, IdeControllerMode::IsaCompatibilityMode)),
                        0x85 => Some((false, true, true, IdeControllerMode::PciNativeMode)),
                        0x8A => Some((true, true, true, IdeControllerMode::IsaCompatibilityMode)),
                        0x8F => Some((true, true, true, IdeControllerMode::PciNativeMode)),
                        _ => None,
                    } {
                        Ok(Self::MassStorageController(
                            MassStorageControllerType::IdeController {
                                supports_isa_mode,
                                supports_pci_mode,
                                supports_bus_mastering,
                                mode,
                            },
                        ))
                    } else {
                        Err((class_code, subclass_code, prog_if))
                    }
                }
                SubClass::MassStorageController::FLOPPY_DISK_CONTROLLER => Ok(
                    Self::MassStorageController(MassStorageControllerType::FloppyDiskController),
                ),
                SubClass::MassStorageController::IPI_BUS_CONTROLLER => Ok(
                    Self::MassStorageController(MassStorageControllerType::IpiBusController),
                ),
                SubClass::MassStorageController::RAID_CONTROLLER => Ok(
                    Self::MassStorageController(MassStorageControllerType::RaidController),
                ),
                SubClass::MassStorageController::ATA_CONTROLLER => {
                    if let Some(dma_mode) = match prog_if {
                        0x20 => Some(AtaControllerDmaMode::SingleDma),
                        0x30 => Some(AtaControllerDmaMode::ChainedDma),
                        _ => None,
                    } {
                        Ok(Self::MassStorageController(
                            MassStorageControllerType::AtaController { dma_mode },
                        ))
                    } else {
                        Err((class_code, subclass_code, prog_if))
                    }
                }
                SubClass::MassStorageController::SERIAL_ATA_CONTROLLER => {
                    if let Some(interface) = match prog_if {
                        0x0 => Some(SataControllerInterface::VendorSpecific),
                        0x1 => Some(SataControllerInterface::AHCI),
                        0x2 => Some(SataControllerInterface::SerialStorageBus),
                        _ => None,
                    } {
                        Ok(Self::MassStorageController(
                            MassStorageControllerType::SataController { interface },
                        ))
                    } else {
                        Err((class_code, subclass_code, prog_if))
                    }
                }
                SubClass::MassStorageController::SERIAL_ATTACHED_SCSI_CONTROLLER => todo!(),
                SubClass::MassStorageController::NON_VOLATILE_MEMORY_CONTROLLER => todo!(),
                SubClass::MassStorageController::OTHER => todo!(),
                _ => Err((class_code, subclass_code, prog_if)),
            },
            ClassCode::NETWORK_CONTROLLER => Ok(Self::NetworkController),
            ClassCode::DISPLAY_CONTROLLER => Ok(Self::DisplayController),
            ClassCode::MULTIMEDIA_CONTROLLER => Ok(Self::MultimediaController),
            ClassCode::MEMORY_CONTROLLER => Ok(Self::MemoryController),
            ClassCode::BRIDGE => Ok(Self::Bridge),
            ClassCode::SIMPLE_COMMUNICATION_CONTROLLER => Ok(Self::SimpleCommunicationController),
            ClassCode::BASE_SYSTEM_PERIPHERAL => Ok(Self::BaseSystemPeripheral),
            ClassCode::INPUT_DEVICE_CONTROLLER => Ok(Self::InputDeviceController),
            ClassCode::DOCKING_STATION => Ok(Self::DockingStation),
            ClassCode::PROCESSOR => Ok(Self::Processor),
            ClassCode::SERIAL_BUS_CONTROLLER => Ok(Self::SerialBusController),
            ClassCode::WIRELESS_CONTROLLER => Ok(Self::WirelessController),
            ClassCode::INTELLIGENT_CONTROLLER => Ok(Self::IntelligentController),
            ClassCode::SATELLITE_COMMUNICATION_CONTROLLER => {
                Ok(Self::SatelliteCommunicationController)
            }
            ClassCode::ENCRYPTION_CONTROLLER => Ok(Self::EncryptionController),
            ClassCode::SIGNAL_PROCESSING_CONTROLLER => Ok(Self::SignalProcessingController),
            ClassCode::PROCESSING_ACCELERATOR => Ok(Self::ProcessingAccelerator),
            ClassCode::NON_ESSENTIAL_INSTRUMENTATION => Ok(Self::NonEssentialInstrumentation),
            0x14..=0x3F => Err((class_code, subclass_code, prog_if)),
            ClassCode::CO_PROCESSOR => Ok(Self::CoProcessor),
            0x41..=0xFE => Err((class_code, subclass_code, prog_if)),
            ClassCode::UNASSIGNED_CLASS => Ok(Self::UnassignedClass),
        }
    }
}

impl ClassCode {
    pub const UNCLASSIFIED: u8 = 0x0;
    pub const MASS_STORAGE_CONTROLLER: u8 = 0x1;
    pub const NETWORK_CONTROLLER: u8 = 0x2;
    pub const DISPLAY_CONTROLLER: u8 = 0x3;
    pub const MULTIMEDIA_CONTROLLER: u8 = 0x4;
    pub const MEMORY_CONTROLLER: u8 = 0x5;
    pub const BRIDGE: u8 = 0x6;
    pub const SIMPLE_COMMUNICATION_CONTROLLER: u8 = 0x7;
    pub const BASE_SYSTEM_PERIPHERAL: u8 = 0x8;
    pub const INPUT_DEVICE_CONTROLLER: u8 = 0x9;
    pub const DOCKING_STATION: u8 = 0xA;
    pub const PROCESSOR: u8 = 0xB;
    pub const SERIAL_BUS_CONTROLLER: u8 = 0xC;
    pub const WIRELESS_CONTROLLER: u8 = 0xD;
    pub const INTELLIGENT_CONTROLLER: u8 = 0xE;
    pub const SATELLITE_COMMUNICATION_CONTROLLER: u8 = 0xF;
    pub const ENCRYPTION_CONTROLLER: u8 = 0x10;
    pub const SIGNAL_PROCESSING_CONTROLLER: u8 = 0x11;
    pub const PROCESSING_ACCELERATOR: u8 = 0x12;
    pub const NON_ESSENTIAL_INSTRUMENTATION: u8 = 0x13;
    pub const CO_PROCESSOR: u8 = 0x40;
    pub const UNASSIGNED_CLASS: u8 = 0xFF;
}

pub struct SubclassUnclassified;

impl SubclassUnclassified {
    pub const NON_VGA_COMPATIBLE_UNCLASSIFIED_DEVICE: u8 = 0x0;
    pub const VGA_COMPATIBLE_UNCLASSIFIED_DEVICE: u8 = 0x1;
}

#[derive(Debug, Clone, Copy)]
pub enum MassStorageControllerType {
    ScsiBusController,
    IdeController {
        supports_isa_mode: bool,
        supports_pci_mode: bool,
        supports_bus_mastering: bool,
        mode: IdeControllerMode,
    },
    FloppyDiskController,
    IpiBusController,
    RaidController,
    AtaController {
        dma_mode: AtaControllerDmaMode,
    },
    SataController {
        interface: SataControllerInterface,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum IdeControllerMode {
    IsaCompatibilityMode,
    PciNativeMode,
}

#[derive(Debug, Clone, Copy)]
pub enum AtaControllerDmaMode {
    SingleDma,
    ChainedDma,
}

#[derive(Debug, Clone, Copy)]
pub enum SataControllerInterface {
    VendorSpecific,
    AHCI,
    SerialStorageBus,
}
