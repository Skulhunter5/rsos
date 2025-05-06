use rsos::pci::{DeviceType, MassStorageControllerType, PciDevice, SataControllerInterface};

#[derive(Debug)]
pub struct AhciController {
    device: PciDevice,
}

impl AhciController {
    pub fn try_from(device: PciDevice) -> Option<Self> {
        match device.device_type() {
            Ok(DeviceType::MassStorageController(MassStorageControllerType::SataController {
                interface: SataControllerInterface::Ahci,
            })) => {},
            Ok(_) | Err(_) => return None,
        }

        todo!();

        Some(Self { device })
    }
}
