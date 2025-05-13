use core::marker::PhantomData;

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

        let command_list = alloc::vec::Vec::<CommandHeader>::with_capacity(32);
        for i in 0..32 {
            todo!();
        }

        Some(Self { device, abar })
    }

    pub fn generic_host_control(&self) -> GenericHostControl {
        GenericHostControl::new(self)
    }

    pub fn ghc_cap(&self) -> u32 {
        let ptr = self.abar as *const u32;
        let cap = unsafe { ptr.read_volatile() };
        cap
    }

    pub fn ghc_ghc(&self) -> u32 {
        let ptr = unsafe { self.abar.add(4) } as *const u32;
        let ghc = unsafe { ptr.read_volatile() };
        ghc
    }
}

#[derive(Debug)]
pub struct GenericHostControl<'a> {
    //controller: &'a AhciController,
    base_ptr: *const u8,
    marker: PhantomData<&'a AhciController>,
    //abar: *const u32,
    //device: &'a PciDevice,
}

impl<'a> GenericHostControl<'a> {
    fn new(controller: &'a AhciController) -> GenericHostControl<'a> {
        let base_ptr = controller.abar;
        Self { base_ptr, marker: PhantomData }
    }

    pub fn capabilities(&self) -> HostCapabilities {
        let ptr = unsafe { self.base_ptr.add(0) } as *const u32;
        return HostCapabilities(unsafe { ptr.read_volatile() });
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HostCapabilities(u32);

macro_rules! host_capabilities{
    {$($capability:ident: $bit:expr),* $(,)?} => {
        #[allow(unused)]
        impl HostCapabilities {
            $(
                pub fn $capability(&self) -> bool {
                    self.0 & (1 << $bit) != 0
                }
            )*
        }
    }
}

host_capabilities! {
    s64a: 31, // Supports 64-Bit Addressing
    sncq: 30, // Supports Native Command Queuing
    ssntf: 29, // Supports SNotification Register
    smps: 28, // Supports Mechanical Presence Switch
    sss: 27, // Supports Staggered Spin-up
    salp: 26, // Supports Aggressive Link Power Management
    sal: 25, // Supports Activity LED
    sclo: 24, // Supports Command List Override
    sam: 18, // Supports AHCI mode only
    spm: 17, // Supports Port Multiplier
    fbss: 16, // FIS-based Switching Supported
    pmd: 15, // PIO Multiple DRQ Block
    ssc: 14, // Slumer State Capable
    psc: 13, // Partial State Capable
    cccs: 7, // Command Completion Coalescing Supported
    ems: 6, // Enclosure Management Supported
    sxs: 5, // Supported External SATA
}

#[allow(unused)]
impl HostCapabilities {
    pub fn iss(&self) -> Option<InterfaceSpeed> {
        let byte = ((self.0 >> 20) & 0b1111) as u8;
        InterfaceSpeed::try_from(byte).ok()
    }

    pub fn ncs(&self) -> u8 {
        ((self.0 >> 8) & 0b11111) as u8
    }

    pub fn command_slot_count(&self) -> u8 {
        self.ncs() + 1
    }

    pub fn np(&self) -> u8 {
        ((self.0 >> 0) & 0b11111) as u8
    }

    pub fn port_count(&self) -> u8 {
        self.np() + 1
    }
}

// TODO: maybe add PartialOrd and Ord
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceSpeed {
    Gen1,
    Gen2,
    Gen3,
}

impl TryFrom<u8> for InterfaceSpeed {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0b0001 => Ok(Self::Gen1),
            0b0010 => Ok(Self::Gen2),
            0b0011 => Ok(Self::Gen3),
            x => Err(x),
        }
    }
}

#[derive(Debug)]
#[repr(C, packed)]
pub struct CommandHeader {
    flags: u16,
    prdtl: u16,
    prdbc: u32,
    ctba: u32,
    ctbau: u32,
    reserved: [u32; 4],
}
