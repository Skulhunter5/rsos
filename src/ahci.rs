use core::marker::PhantomData;

use rsos::pci::{DeviceType, MassStorageControllerType, PciDevice, SataControllerInterface};

#[derive(Debug)]
pub struct AhciController {
    device: PciDevice,
    abar: *const (),
}

impl AhciController {
    pub fn try_from(device: PciDevice) -> Option<Self> {
        match device.device_type() {
            Ok(DeviceType::MassStorageController(MassStorageControllerType::SataController {
                interface: SataControllerInterface::Ahci,
            })) => {},
            Ok(_) | Err(_) => return None,
        }

        let abar = device.bar5() as *const ();
        if abar.is_null() {
            return None;
        }

        let mut controller = Self { device, abar };
        controller.init();

        let caps = controller.generic_host_control().capabilities();
        crate::println!("CAP.SAM: {}", caps.sam());

        crate::println!("Beginning HBA reset...");
        controller.generic_host_control().global_hba_control().reset_and_wait();
        crate::println!("> reset complete");

        Some(controller)
    }

    fn init(&mut self) {
        let port_count = self.generic_host_control().capabilities().port_count() as usize;
        for port in 0..port_count {
            let command_list = alloc::vec![CommandHeader {
                flags: 0,
                prdtl: 0,
                prdbc: 0,
                ctba: 0,
                ctbau: 0,
                reserved: [0; 4],
            }; 32].into_boxed_slice();

            let received_fis = alloc::vec![0; 4096].into_boxed_slice();

            self.init_port(port);
        }
    }

    fn init_port(&mut self, port: usize) {
        let mut registers = self.port_register(port);
        crate::println!("Port {} ST: {}", port, registers.cmd().st());
    }

    pub fn get_port(&mut self, port: usize) -> Port {
        todo!();
    }

    pub fn generic_host_control(&mut self) -> GenericHostControl {
        GenericHostControl::new(self)
    }

    fn port_register(&mut self, port: usize) -> PortRegisters {
        PortRegisters::new(self, port)
    }
}

#[derive(Debug)]
pub struct GenericHostControl<'a> {
    base_ptr: *const (),
    _marker: PhantomData<&'a mut AhciController>,
}

#[allow(unused)]
impl<'a> GenericHostControl<'a> {
    fn new(controller: &'a mut AhciController) -> GenericHostControl<'a> {
        let base_ptr = controller.abar;
        Self { base_ptr, _marker: PhantomData }
    }

    pub fn capabilities(&self) -> HostCapabilities {
        let ptr = unsafe { self.base_ptr.byte_add(0) } as *const u32;
        return HostCapabilities(unsafe { ptr.read_volatile() });
    }

    pub fn global_hba_control(&mut self) -> GlobalHbaControl {
        GlobalHbaControl::new(self)
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
pub struct GlobalHbaControl<'a> {
    ptr: *mut u32,
    _marker: PhantomData<&'a mut GenericHostControl<'a>>,
}

#[allow(unused)]
impl<'a> GlobalHbaControl<'a> {
    fn new(ghc: &GenericHostControl) -> Self {
        let ptr = unsafe { ghc.base_ptr.byte_add(0x4) } as *mut u32;
        Self { ptr, _marker: PhantomData }
    }

    fn read(&self) -> u32 {
        unsafe { self.ptr.read_volatile() }
    }

    fn write(&mut self, val: u32) {
        unsafe { self.ptr.write_volatile(val) }
    }

    pub fn ae(&self) -> bool {
        self.read() & (1 << 31) != 0
    }

    pub fn enable_ahci_mode(&mut self) {
        if self.ae() {
            return;
        }

        // According to the spec, you shall not access any other AHCI registers when not in ahci
        // mode, therefore there shouldn't be any flags set previously to enabling ahci mode
        self.write(1 << 31);
    }

    pub fn disable_ahci_mode(&mut self) {
        if !self.ae() {
            return;
        }

        // According to the spec, for disabling ahci mode, you shall write 0 to the register
        self.write(0);
    }

    pub fn mrsm(&self) -> bool {
        self.read() & (1 << 2) != 0
    }

    pub fn ie(&self) -> bool {
        self.read() & (1 << 1) != 0
    }

    pub fn enable_interrupts(&mut self) {
        self.write(self.read() | (1 << 1));
    }

    pub fn disable_interrupts(&mut self) {
        self.write(self.read() & !(1 << 1));
    }

    pub fn hr(&self) -> bool {
        self.read() & (1 << 0) != 0
    }

    pub fn reset(&mut self) {
        self.write(self.read() | (1 << 0));
    }

    pub fn reset_and_wait(&mut self) {
        self.reset();
        while self.hr() {
            core::hint::spin_loop();
        }
    }
}

#[derive(Debug)]
pub struct Port {
}

#[derive(Debug)]
struct PortRegisters<'a> {
    base_ptr: *const (),
    _marker: PhantomData<&'a mut AhciController>
}

#[allow(unused)]
impl PortRegisters<'_> {
    fn new(controller: &mut AhciController, port: usize) -> Self {
        let base_ptr = unsafe { controller.abar.byte_add(0x100 + 0x80 * port) };
        Self { base_ptr, _marker: PhantomData }
    }

    pub fn cmd(&mut self) -> PortCmd {
        PortCmd::new(self)
    }
}

#[derive(Debug)]
struct PortCmd<'a> {
    ptr: *mut u32,
    _marker: PhantomData<&'a mut AhciController>,
}

#[allow(unused)]
impl PortCmd<'_> {
    fn new(registers: &mut PortRegisters) -> Self {
        let ptr = unsafe { registers.base_ptr.byte_add(0x18) } as *mut u32;
        Self { ptr, _marker: PhantomData }
    }

    fn read(&self) -> u32 {
        unsafe { self.ptr.read_volatile() }
    }

    fn write(&mut self, val: u32) {
        unsafe { self.ptr.write_volatile(val) }
    }

    const ST_BIT: u32 = 1 << 0;

    pub fn st(&self) -> bool {
        self.read() & Self::ST_BIT != 0
    }

    pub fn start(&mut self) {
        self.write(self.read() | Self::ST_BIT);
    }

    pub fn stop(&mut self) {
        self.write(self.read() & !Self::ST_BIT);
    }

    const FRE_BIT: u32 = 1 << 4;

    pub fn fre(&self) -> bool {
        self.read() & Self::FRE_BIT != 0
    }

    pub fn enable_fre(&mut self) {
        self.write(self.read() | Self::FRE_BIT);
    }

    pub fn disable_fre(&mut self) {
        self.write(self.read() & !Self::FRE_BIT);
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct CommandHeader {
    flags: u16,
    prdtl: u16,
    prdbc: u32,
    ctba: u32,
    ctbau: u32,
    reserved: [u32; 4],
}

type CommandFis = [u8; 64];
type AtapiCommand = [u8; 16];

// #[derive(Debug)]
#[repr(C, packed)]
struct CommandTable {
    cfis: CommandFis,
    acmd: AtapiCommand,
    reserved: [u8; 0x30],
    prdts: [Prdt],
}

#[derive(Debug)]
#[repr(C, packed)]
struct Prdt {
    dba: u32,
    dbau: u32,
    reserved: u32,
    dw3: u32,
}
