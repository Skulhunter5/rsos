use super::SubClass;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Unclassified { vga_compatible: bool },
    MassStorageController(MassStorageControllerType),
    NetworkController(NetworkControllerType),
    DisplayController(DisplayControllerType),
    MultimediaController(MultimediaControllerType),
    MemoryController(MemoryControllerType),
    Bridge(BridgeType),
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
                SubClass::Unclassified::NON_VGA_COMPATIBLE_UNCLASSIFIED_DEVICE => {
                    Ok(Self::Unclassified {
                        vga_compatible: false,
                    })
                }
                SubClass::Unclassified::VGA_COMPATIBLE_UNCLASSIFIED_DEVICE => {
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
                        0x1 => Some(SataControllerInterface::Ahci),
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
                SubClass::MassStorageController::SERIAL_ATTACHED_SCSI_CONTROLLER => {
                    if let Some(interface) = match prog_if {
                        0x0 => Some(SasControllerInterface::Sas),
                        0x1 => Some(SasControllerInterface::SerialStorageBus),
                        _ => None,
                    } {
                        Ok(Self::MassStorageController(
                            MassStorageControllerType::SasController { interface },
                        ))
                    } else {
                        Err((class_code, subclass_code, prog_if))
                    }
                }
                SubClass::MassStorageController::NON_VOLATILE_MEMORY_CONTROLLER => {
                    if let Some(interface) = match prog_if {
                        0x1 => Some(NvmControllerInterface::Nvmhci),
                        0x2 => Some(NvmControllerInterface::Nvme),
                        _ => None,
                    } {
                        Ok(Self::MassStorageController(
                            MassStorageControllerType::NvmController { interface },
                        ))
                    } else {
                        Err((class_code, subclass_code, prog_if))
                    }
                }
                SubClass::MassStorageController::OTHER => Ok(Self::MassStorageController(
                    MassStorageControllerType::Other,
                )),
                _ => Err((class_code, subclass_code, prog_if)),
            },
            ClassCode::NETWORK_CONTROLLER => match subclass_code {
                SubClass::NetworkController::ETHERNET_CONTROLLER => Ok(Self::NetworkController(
                    NetworkControllerType::EthernetController,
                )),
                SubClass::NetworkController::TOKEN_RING_CONTROLLER => Ok(Self::NetworkController(
                    NetworkControllerType::TokenRingController,
                )),
                SubClass::NetworkController::FDDI_CONTROLLER => Ok(Self::NetworkController(
                    NetworkControllerType::FddiController,
                )),
                SubClass::NetworkController::ATM_CONTROLLER => Ok(Self::NetworkController(
                    NetworkControllerType::AtmController,
                )),
                SubClass::NetworkController::ISDN_CONTROLLER => Ok(Self::NetworkController(
                    NetworkControllerType::IsdnController,
                )),
                SubClass::NetworkController::WORLD_FIP_CONTROLLER => Ok(Self::NetworkController(
                    NetworkControllerType::WorldFipController,
                )),
                SubClass::NetworkController::PICMG_214_MULTI_COMPUTING_CONTROLLER => {
                    Ok(Self::NetworkController(
                        NetworkControllerType::Picmg214MultiComputingController,
                    ))
                }
                SubClass::NetworkController::INFINIBAND_CONTROLLER => Ok(Self::NetworkController(
                    NetworkControllerType::InfinibandController,
                )),
                SubClass::NetworkController::FABRIC_CONTROLLER => Ok(Self::NetworkController(
                    NetworkControllerType::FabricController,
                )),
                SubClass::NetworkController::OTHER => {
                    Ok(Self::NetworkController(NetworkControllerType::Other))
                }
                _ => Err((class_code, subclass_code, prog_if)),
            },
            ClassCode::DISPLAY_CONTROLLER => match subclass_code {
                SubClass::DisplayController::VGA_COMPATIBLE_CONTROLLER => {
                    if let Some(ty) = match prog_if {
                        0x0 => Some(VgaCompatibleControllerType::VgaController),
                        0x1 => Some(VgaCompatibleControllerType::Controller8514Compatible),
                        _ => None,
                    } {
                        Ok(Self::DisplayController(
                            DisplayControllerType::VgaCompatibleController(ty),
                        ))
                    } else {
                        Err((class_code, subclass_code, prog_if))
                    }
                }
                SubClass::DisplayController::XGA_CONTROLLER => Ok(Self::DisplayController(
                    DisplayControllerType::XgaController,
                )),
                SubClass::DisplayController::CONTROLLER_3D => {
                    Ok(Self::DisplayController(DisplayControllerType::Controller3D))
                }
                SubClass::DisplayController::OTHER => {
                    Ok(Self::DisplayController(DisplayControllerType::Other))
                }
                _ => Err((class_code, subclass_code, prog_if)),
            },
            ClassCode::MULTIMEDIA_CONTROLLER => match subclass_code {
                SubClass::MultimediaController::MULTIMEDIA_VIDEO_CONTROLLER => Ok(
                    Self::MultimediaController(MultimediaControllerType::MultimediaVideoController),
                ),
                SubClass::MultimediaController::MULTIMEDIA_AUDIO_CONTROLLER => Ok(
                    Self::MultimediaController(MultimediaControllerType::MultimediaAudioController),
                ),
                SubClass::MultimediaController::COMPUTER_TELEPHONY_DEVICE => Ok(
                    Self::MultimediaController(MultimediaControllerType::ComputerTelephonyDevice),
                ),
                SubClass::MultimediaController::AUDIO_DEVICE => Ok(Self::MultimediaController(
                    MultimediaControllerType::AudioDevice,
                )),
                SubClass::MultimediaController::OTHER => {
                    Ok(Self::MultimediaController(MultimediaControllerType::Other))
                }
                _ => Err((class_code, subclass_code, prog_if)),
            },
            ClassCode::MEMORY_CONTROLLER => match subclass_code {
                SubClass::MemoryController::RAM_CONTROLLER => {
                    Ok(Self::MemoryController(MemoryControllerType::RamController))
                }
                SubClass::MemoryController::FLASH_CONTROLLER => Ok(Self::MemoryController(
                    MemoryControllerType::FlashController,
                )),
                SubClass::MemoryController::OTHER => {
                    Ok(Self::MemoryController(MemoryControllerType::Other))
                }
                _ => Err((class_code, subclass_code, prog_if)),
            },
            ClassCode::BRIDGE => match subclass_code {
                SubClass::Bridge::HOST_BRIDGE => Ok(Self::Bridge(BridgeType::HostBridge)),
                SubClass::Bridge::ISA_BRIDGE => Ok(Self::Bridge(BridgeType::IsaBridge)),
                SubClass::Bridge::EISA_BRIDGE => Ok(Self::Bridge(BridgeType::EisaBridge)),
                SubClass::Bridge::MCA_BRIDGE => Ok(Self::Bridge(BridgeType::McaBridge)),
                SubClass::Bridge::PCI_TO_PCI_BRIDGE => {
                    if let Some(decode_mode) = match prog_if {
                        0x0 => Some(PciBridgeDecodeMode::NormalDecode),
                        0x1 => Some(PciBridgeDecodeMode::SubtractiveDecode),
                        _ => None,
                    } {
                        Ok(Self::Bridge(BridgeType::PciToPciBridge { decode_mode }))
                    } else {
                        Err((class_code, subclass_code, prog_if))
                    }
                }
                SubClass::Bridge::PCMCIA_BRIDGE => Ok(Self::Bridge(BridgeType::PcmciaBridge)),
                SubClass::Bridge::NUBUS_BRIDGE => Ok(Self::Bridge(BridgeType::NuBusBridge)),
                SubClass::Bridge::CARDBUS_BRIDGE => Ok(Self::Bridge(BridgeType::CardBusBridge)),
                SubClass::Bridge::RACEWAY_BRIDGE => {
                    if let Some(mode) = match prog_if {
                        0x0 => Some(RaceWayBridgeMode::Transparent),
                        0x1 => Some(RaceWayBridgeMode::Endpoint),
                        _ => None,
                    } {
                        Ok(Self::Bridge(BridgeType::RaceWayBridge { mode }))
                    } else {
                        Err((class_code, subclass_code, prog_if))
                    }
                }
                SubClass::Bridge::SEMI_TRANSPARENT_PCI_TO_PCI_BRIDGE => {
                    if let Some(cpu_facing_bus) = match prog_if {
                        0x40 => Some(CpuFacingBus::Primary),
                        0x80 => Some(CpuFacingBus::Secondary),
                        _ => None,
                    } {
                        Ok(Self::Bridge(BridgeType::SemiTransparentPciToPciBridge {
                            cpu_facing_bus,
                        }))
                    } else {
                        Err((class_code, subclass_code, prog_if))
                    }
                }
                SubClass::Bridge::INFINIBAND_TO_PCI_HOST_BRIDGE => {
                    Ok(Self::Bridge(BridgeType::InfinibandToPciHostBridge))
                }
                SubClass::Bridge::OTHER => Ok(Self::Bridge(BridgeType::Other)),
                _ => Err((class_code, subclass_code, prog_if)),
            },
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

pub struct ClassCode;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    SasController {
        interface: SasControllerInterface,
    },
    NvmController {
        interface: NvmControllerInterface,
    },
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdeControllerMode {
    IsaCompatibilityMode,
    PciNativeMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtaControllerDmaMode {
    SingleDma,
    ChainedDma,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SataControllerInterface {
    VendorSpecific,
    Ahci,
    SerialStorageBus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SasControllerInterface {
    Sas,
    SerialStorageBus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NvmControllerInterface {
    Nvmhci,
    Nvme,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkControllerType {
    EthernetController,
    TokenRingController,
    FddiController,
    AtmController,
    IsdnController,
    WorldFipController,
    Picmg214MultiComputingController,
    InfinibandController,
    FabricController,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayControllerType {
    VgaCompatibleController(VgaCompatibleControllerType),
    XgaController,
    Controller3D,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VgaCompatibleControllerType {
    VgaController,
    Controller8514Compatible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultimediaControllerType {
    MultimediaVideoController,
    MultimediaAudioController,
    ComputerTelephonyDevice,
    AudioDevice,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryControllerType {
    RamController,
    FlashController,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeType {
    HostBridge,
    IsaBridge,
    EisaBridge,
    McaBridge,
    PciToPciBridge { decode_mode: PciBridgeDecodeMode },
    PcmciaBridge,
    NuBusBridge,
    CardBusBridge,
    RaceWayBridge { mode: RaceWayBridgeMode },
    SemiTransparentPciToPciBridge { cpu_facing_bus: CpuFacingBus },
    InfinibandToPciHostBridge,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PciBridgeDecodeMode {
    NormalDecode,
    SubtractiveDecode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaceWayBridgeMode {
    Transparent,
    Endpoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuFacingBus {
    Primary,
    Secondary,
}
