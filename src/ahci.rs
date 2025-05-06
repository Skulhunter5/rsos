use rsos::pci::{DeviceType, MassStorageControllerType, PciDevice, SataControllerInterface};

#[derive(Debug)]
pub struct AhciController {
    device: PciDevice,
    abar: *const u8,
}

impl AhciController {
    pub fn try_from(device: PciDevice) -> Option<Self> {
        match device.device_type() {
            Ok(DeviceType::MassStorageController(MassStorageControllerType::SataController {
                interface: SataControllerInterface::Ahci,
            })) => {},
            Ok(_) | Err(_) => return None,
        }

        let abar = device.bar5() as *const u8;
        if abar.is_null() {
            return None;
        }

        Some(Self { device, abar })
    }

    pub fn ghc_cap(&self) -> u32 {
        let ptr = self.abar as *const u32;
        let cap = unsafe { *ptr };
        cap
    }

    pub fn ghc_ghc(&self) -> u32 {
        let ptr = self.abar as *const u32;
        let ghc = unsafe { *ptr };
        ghc
    }
}
