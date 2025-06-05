use core::{marker::PhantomData, mem::MaybeUninit};

use alloc::boxed::Box;
use rsos::pci::{DeviceType, MassStorageControllerType, PciDevice, SataControllerInterface};

#[derive(Debug)]
pub struct AhciController {
    device: PciDevice,
    abar: *const (),
    capabilities: HostCapabilities,
    ports: [Option<Port>; 32],
}

impl AhciController {
    pub fn try_from(device: PciDevice) -> Result<Self, &'static str> {
        match device.device_type() {
            Ok(DeviceType::MassStorageController(MassStorageControllerType::SataController {
                interface: SataControllerInterface::Ahci,
            })) => {}
            Ok(_) | Err(_) => return Err("incorrect pci device type"),
        }

        let abar = device.bar5() as *const ();
        if abar.is_null() {
            return Err("abar is null");
        }

        let capabilities = Self::read_caps(abar);
        if capabilities.fbss() {
            return Err("invalid device configuration: CAP.FBSS == true");
        }

        let ports = [const { None }; 32];

        let mut controller = Self {
            device,
            abar,
            ports,
            capabilities,
        };

        crate::println!("Beginning HBA reset...");
        controller
            .generic_host_control()
            .global_hba_control()
            .reset_and_wait();
        crate::println!("> reset complete");

        // Make sure that AHCI mode is enabled
        if !capabilities.sam() {
            let mut ghc = controller.generic_host_control();
            ghc.global_hba_control().enable_ahci_mode();
        }

        controller.init();

        Ok(controller)
    }

    fn read_caps(abar: *const ()) -> HostCapabilities {
        HostCapabilities(unsafe { (abar as *const u32).read_volatile() })
    }

    fn init(&mut self) {
        let port_count = self.generic_host_control().capabilities().port_count() as usize;
        let pi = self.generic_host_control().ports_implemented();
        for port in 0..port_count {
            if !pi.is_port_implemented(port) {
                continue;
            }

            self.init_port(port);
        }
    }

    fn init_port(&mut self, index: usize) {
        let mut registers = unsafe { self.port_registers(index) };
        crate::println!("Port {} ST: {}", index, registers.cmd().st());
        let mut port = Port::init(self, index);

        let command_list = CommandList::new();
        let fis_receive_area = Box::new(FisReceiveArea::new());
        port.setup(
            command_list.get_address(),
            &*fis_receive_area as *const FisReceiveArea as u64,
        );

        self.ports[index] = Some(port);
    }

    pub fn get_port(&mut self, port: usize) -> Option<&mut Port> {
        if port >= self.ports.len() {
            return None;
        }
        if let Some(ref mut port) = self.ports[port] {
            return Some(port);
        } else {
            return None;
        }
    }

    pub fn generic_host_control(&mut self) -> GenericHostControl {
        GenericHostControl::new(self)
    }

    unsafe fn port_registers(&mut self, port: usize) -> PortRegisters {
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
    const OFFSET_GHC: usize = 0;
    const OFFSET_PI: usize = 0xC;

    fn new(controller: &'a mut AhciController) -> GenericHostControl<'a> {
        let base_ptr = controller.abar;
        Self {
            base_ptr,
            _marker: PhantomData,
        }
    }

    pub fn capabilities(&self) -> HostCapabilities {
        let ptr = unsafe { self.base_ptr.byte_add(Self::OFFSET_GHC) } as *const u32;
        return HostCapabilities(unsafe { ptr.read_volatile() });
    }

    pub fn ports_implemented(&self) -> ImplementedPorts {
        let ptr = unsafe {
            self.base_ptr
                .byte_add(Self::OFFSET_GHC)
                .byte_add(Self::OFFSET_PI)
        } as *const u32;
        return ImplementedPorts(unsafe { ptr.read_volatile() });
    }

    pub fn global_hba_control(&mut self) -> GlobalHbaControl {
        GlobalHbaControl::new(self)
    }
}

// TODO: create custom Debug/Display impl
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

// TODO: create custom Debug/Display impl
#[derive(Debug, Clone, Copy)]
pub struct ImplementedPorts(u32);

#[allow(unused)]
impl ImplementedPorts {
    pub fn is_port_implemented(&self, index: usize) -> bool {
        self.0 & (1 << index) != 0
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
        Self {
            ptr,
            _marker: PhantomData,
        }
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
    base_ptr: *const (),
    s64a: bool,
}

impl Port {
    fn init(controller: &AhciController, port: usize) -> Self {
        let base_ptr = unsafe { controller.abar.byte_add(0x100 + 0x80 * port) };
        let s64a = controller.capabilities.s64a();

        Self { base_ptr, s64a }
    }

    fn set_clb(&mut self, address: u64) {
        let lower = address as u32;
        let upper = (address >> 32) as u32;

        if lower & 0b11_1111_1111 != 0 {
            panic!(
                "tried to write {} to PxCLB(U); must be aligned to 1K bytes",
                address
            );
        }

        let clb_ptr = self.base_ptr as *mut u32;
        unsafe { clb_ptr.write_volatile(lower) };
        if self.s64a {
            let clbu_ptr = unsafe { clb_ptr.byte_add(0x4) } as *mut u32;
            unsafe {
                clbu_ptr.write_volatile(upper);
            }
        } else if upper != 0 {
            panic!(
                "tried to write {} (> 4 GiB) to PxCLB(U) with CAP.S64A == false",
                address
            );
        }
    }

    fn set_fb(&mut self, address: u64) {
        let lower = address as u32;
        let upper = (address >> 32) as u32;

        if lower & 0b1111_1111 != 0 {
            panic!(
                "tried to write {} to PxFB(U); must be aligned to 256 bytes",
                address
            );
        }

        let fb_ptr = self.base_ptr as *mut u32;
        unsafe { fb_ptr.write_volatile(lower) };
        if self.s64a {
            let fbu_ptr = unsafe { fb_ptr.byte_add(0x4) } as *mut u32;
            unsafe {
                fbu_ptr.write_volatile(upper);
            }
        } else if upper != 0 {
            panic!(
                "tried to write {} (> 4 GiB) to PxFB(U) with CAP.S64A == false",
                address
            );
        }
    }

    fn registers(&mut self) -> PortRegisters {
        PortRegisters {
            base_ptr: self.base_ptr,
            _marker: PhantomData,
        }
    }

    fn start(&mut self) {
        let mut registers = self.registers();
        let mut cmd = registers.cmd();
        cmd.enable_fre();
        cmd.start();
    }

    fn stop(&mut self) {
        let mut registers = self.registers();
        let mut cmd = registers.cmd();
        cmd.stop();
        cmd.disable_fre();
    }

    fn setup(&mut self, clb: u64, fb: u64) {
        self.stop();

        self.set_clb(clb);
        self.set_fb(fb);

        self.start();
    }
}

#[derive(Debug)]
struct PortRegisters<'a> {
    base_ptr: *const (),
    _marker: PhantomData<&'a mut AhciController>,
}

#[allow(unused)]
impl PortRegisters<'_> {
    fn new(controller: &mut AhciController, port: usize) -> Self {
        let base_ptr = unsafe { controller.abar.byte_add(0x100 + 0x80 * port) };
        Self {
            base_ptr,
            _marker: PhantomData,
        }
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
        Self {
            ptr,
            _marker: PhantomData,
        }
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

    pub fn disable_fis_receive(&mut self) {
        self.write(self.read() & !Self::FRE_BIT);
    }

    pub fn disable_fis_receive_and_wait(&mut self) {
        self.disable_fis_receive();
        while self.fr() {
            core::hint::spin_loop();
        }
    }

    const FR_BIT: u32 = 1 << 14;

    pub fn fr(&self) -> bool {
        self.read() & Self::FR_BIT != 0
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

#[derive(Debug)]
struct CommandList {
    data: Box<[CommandHeader; 32]>,
}

impl CommandList {
    fn new() -> Self {
        const TEMPLATE: CommandHeader = CommandHeader {
            flags: 0,
            prdtl: 0,
            prdbc: 0,
            ctba: 0,
            ctbau: 0,
            reserved: [0; 4],
        };

        let layout = alloc::alloc::Layout::from_size_align(
            core::mem::size_of::<[CommandHeader; 32]>(),
            1024,
        )
        .unwrap();
        let ptr = unsafe { alloc::alloc::alloc(layout) } as *mut [MaybeUninit<CommandHeader>; 32];
        let mut data = unsafe { Box::from_raw(ptr) };

        for item in data.iter_mut() {
            item.write(TEMPLATE);
        }

        let data = unsafe {
            core::mem::transmute::<Box<[MaybeUninit<CommandHeader>; 32]>, Box<[CommandHeader; 32]>>(
                data,
            )
        };

        Self { data }
    }

    fn get_address(&self) -> u64 {
        self.data.as_ptr() as u64
    }
}

// TODO: change FisReceiveArea size if support for FBSS is added
#[repr(align(256))]
pub struct FisReceiveArea([u8; 256]);

impl FisReceiveArea {
    pub fn new() -> Self {
        Self([0; 256])
    }
}
