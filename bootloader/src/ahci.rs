use core::{
    alloc::Layout,
    fmt::Debug,
    marker::PhantomData,
    mem::{self, MaybeUninit},
    ptr,
};

use alloc::{
    alloc::{alloc, alloc_zeroed},
    boxed::Box,
    format,
    string::String,
    vec::Vec,
};
use bootloader::pci::{DeviceType, MassStorageControllerType, PciDevice, SataControllerInterface};

use crate::disk::StorageDevice;

#[derive(Debug)]
pub struct AhciController {
    device: PciDevice,
    abar: *const (),
    capabilities: HostCapabilities,
    ports: [Option<Port>; 32],
}

impl AhciController {
    pub fn try_from(mut device: PciDevice) -> Result<Self, &'static str> {
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

        if !device.bus_mastering() {
            device.enable_bus_mastering();
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

        // crate::println!("Beginning HBA reset...");
        // controller
        //     .generic_host_control()
        //     .global_hba_control()
        //     .reset_and_wait();
        // crate::println!("> reset complete");

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
        for index in 0..port_count {
            if !pi.is_port_implemented(index) {
                continue;
            }

            let port = Port::init(self, index);
            self.ports[index] = Some(port);
        }
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
    command_list: CommandList,
    fis_receive_area: Box<FisReceiveArea>,
    command_table: Box<CommandTable>,
}

impl Port {
    fn init(controller: &AhciController, port: usize) -> Self {
        let port_index = port;

        let base_ptr = unsafe { controller.abar.byte_add(0x100 + 0x80 * port) };
        let s64a = controller.capabilities.s64a();

        let command_list = CommandList::new();
        let fis_receive_area = Box::new(FisReceiveArea::new());

        let command_table = CommandTable::new([0; 64], [0; 16], &[]);

        let mut port = Self {
            base_ptr,
            s64a,
            command_list,
            fis_receive_area,
            command_table,
        };

        crate::println!("Port {}: {:?}", port_index, port.status());

        port.setup();

        port
    }

    fn setup(&mut self) {
        self.stop();

        self.set_clb(self.command_list.get_address());
        self.set_fb(&*self.fis_receive_area as *const FisReceiveArea as u64);

        self.start();
    }

    fn get_clb(&self) -> u64 {
        let clb_ptr = unsafe { self.base_ptr.byte_add(0) } as *const u32;
        let clbu_ptr = unsafe { clb_ptr.add(1) };
        let clb = unsafe { clb_ptr.read_volatile() } as u64;
        let clbu = unsafe { clbu_ptr.read_volatile() } as u64;
        (clbu << 32) | clb
    }

    fn get_fb(&self) -> u64 {
        let fb_ptr = unsafe { self.base_ptr.byte_add(0x8) } as *const u32;
        let fbu_ptr = unsafe { fb_ptr.add(1) };
        let fb = unsafe { fb_ptr.read_volatile() } as u64;
        let fbu = unsafe { fbu_ptr.read_volatile() } as u64;
        (fbu << 32) | fb
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

        let clb_ptr = unsafe { self.base_ptr.byte_add(0) } as *mut u32;
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

        let fb_ptr = unsafe { self.base_ptr.byte_add(0x8) } as *mut u32;
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

    // TODO: wait after setting CMD.FRE and CMD.ST
    fn start(&mut self) {
        let mut registers = self.registers();
        let mut cmd = registers.cmd();
        cmd.enable_fre();
        cmd.start();
    }

    // TODO: wait after unsetting CMD.ST and CMD.FRE
    fn stop(&mut self) {
        let mut registers = self.registers();
        let mut cmd = registers.cmd();
        cmd.stop();
        cmd.disable_fre();
    }

    fn run_command(&mut self, slot: u8) {
        let bit_mask = 1 << slot;

        let ptr = unsafe { self.base_ptr.byte_add(0x38) } as *mut u32;
        unsafe {
            ptr.write_volatile(bit_mask);
        }
        while unsafe { ptr.read_volatile() } & bit_mask != 0 {
            core::hint::spin_loop();
        }
    }

    pub fn read(
        &mut self,
        buffer: &mut [u8],
        lba: u64,
        sector_count: u16,
    ) -> Result<(), &'static str> {
        let command_slot = 0u8;

        // Build read FIS
        let mut fis = [0; 64];
        {
            fis[0] = FisType::RegisterH2D as u8;
            fis[1] = 0x80;
            fis[2] = 0x25;
            fis[4] = lba as u8;
            fis[5] = (lba >> 8) as u8;
            fis[6] = (lba >> 16) as u8;
            fis[7] = 1 << 6;
            fis[8] = (lba >> 24) as u8;
            fis[9] = (lba >> 32) as u8;
            fis[10] = (lba >> 40) as u8;
            fis[12] = sector_count as u8;
            fis[13] = (sector_count >> 8) as u8;
        }
        let acmd = [0; 16];
        let buffer_address = buffer.as_ptr() as u64;
        // TODO: check that the -1 is correct and figure out why
        assert!(buffer.len() > 0, "Buffer must not be empty");
        let sector_size = 512;
        assert_eq!(
            buffer.len() % sector_size,
            0,
            "Buffer must be sector-aligned"
        );
        let buffer_size = buffer.len() as u32 - 1;
        let prdts = [Prdt::new(buffer_address, buffer_size)];

        // let fis = RegisterFisH2D::new();
        self.command_table = CommandTable::new(fis, acmd, &prdts);

        let command_header = &mut self.command_list.data[command_slot as usize];
        command_header.flags = 5;
        command_header.prdtl = prdts.len() as u16;
        let command_table_address = self.command_table.get_address();
        let ctba = command_table_address as u32;
        command_header.ctba = ctba;
        let ctbau = (command_table_address >> 32) as u32;
        if self.s64a {
            command_header.ctbau = ctbau;
        } else if ctbau != 0 {
            panic!(
                "tried to write {} (> 4 GiB) to CommandHeader.ctba(u) with CAP.S64A == false",
                command_table_address
            );
        }

        self.run_command(command_slot);

        // TODO: add error checking and handling
        Ok(())
    }

    pub fn identify(&mut self) -> Result<DeviceProperties, &'static str> {
        let command_slot = 0u8;

        let buffer = alloc::vec![0u8; 512];

        // Build read FIS
        let mut fis = [0; 64];
        {
            fis[0] = FisType::RegisterH2D as u8;
            fis[1] = 0x80;
            fis[2] = 0xEC;
            fis[7] = 0x40;
        }
        let acmd = [0; 16];
        let buffer_address = buffer.as_ptr() as u64;
        // TODO: check that the -1 is correct and figure out why
        assert!(buffer.len() > 0, "Buffer must not be empty");
        let sector_size = 512;
        assert_eq!(
            buffer.len() % sector_size,
            0,
            "Buffer must be sector-aligned"
        );
        let buffer_size = buffer.len() as u32 - 1;
        let prdts = [Prdt::new(buffer_address, buffer_size)];

        // let fis = RegisterFisH2D::new();
        self.command_table = CommandTable::new(fis, acmd, &prdts);

        let command_header = &mut self.command_list.data[command_slot as usize];
        command_header.flags = 5;
        command_header.prdtl = prdts.len() as u16;
        let command_table_address = self.command_table.get_address();
        let ctba = command_table_address as u32;
        command_header.ctba = ctba;
        let ctbau = (command_table_address >> 32) as u32;
        if self.s64a {
            command_header.ctbau = ctbau;
        } else if ctbau != 0 {
            panic!(
                "tried to write {} (> 4 GiB) to CommandHeader.ctba(u) with CAP.S64A == false",
                command_table_address
            );
        }

        self.run_command(command_slot);
        // TODO: add error checking and handling

        let data = unsafe {
            assert!(buffer.len() >= size_of::<IdentifyDeviceData>());

            let data = MaybeUninit::<IdentifyDeviceData>::uninit();
            buffer.as_ptr().copy_to_nonoverlapping(data.as_ptr() as *mut u8, size_of::<IdentifyDeviceData>());
            data.assume_init()
        };
        
        let sector_count = if data.commands_and_feature_sets_supported1 | (1 << 10) != 0 {
            data.user_addressable_logical_sectors_48 as u64
        } else {
            data.user_addressable_logical_sectors_28 as u64
        };

        let device_properties = DeviceProperties {
            sector_count,
        };
        Ok(device_properties)
    }

    fn status(&self) -> SataStatus {
        const OFFSET_SSTS: usize = 0x28;
        let ptr = unsafe { self.base_ptr.byte_add(OFFSET_SSTS) } as *const u32;
        SataStatus::try_from(unsafe { ptr.read_volatile() }).unwrap()
    }

    fn error(&self) -> SataError {
        const OFFSET_SERR: usize = 0x30;
        let ptr = unsafe { self.base_ptr.byte_add(OFFSET_SERR) } as *const u32;
        SataError(unsafe { ptr.read_volatile() })
    }

    fn tfd(&self) -> TaskFileData {
        const OFFSET_TFD: usize = 0x20;
        let ptr = unsafe { self.base_ptr.byte_add(OFFSET_TFD) } as *const u32;
        TaskFileData(unsafe { ptr.read_volatile() })
    }
}

impl StorageDevice for Port {
    fn read(&mut self, buffer: &mut [u8], lba: u64, sectors: u16) -> Result<(), &'static str> {
        self.read(buffer, lba, sectors)
    }

    fn write(&mut self, _buffer: &[u8], _lba: u64) -> Result<(), &'static str> {
        todo!()
    }

    fn sector_count(&mut self) -> Result<u64, &'static str> {
        let device_properties = self.identify()?;
        Ok(device_properties.sector_count)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DeviceProperties {
    sector_count: u64,
}

// struct based on https://people.freebsd.org/~imp/asiabsdcon2015/works/d2161r5-ATAATAPI_Command_Set_-_3.pdf
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct IdentifyDeviceData {
    pub general_configuration: u16,
    obsolete0: u16,
    pub specific_configuration: u16,
    obsolete1: u16,
    retired0: [u16; 2],
    obsolete2: u16,
    reserved_for_cfa0: [u16; 2],
    retired1: u16,
    pub serial_number: [u8; 20], // 10..19
    retired2: [u16; 2],
    obsolete3: u16,
    pub firmware_revision: [u16; 4], // 23..26
    pub model_number: [u16; 20], // 27..46
    pub other0: u16, // 47
    pub trusted_computing_feature_set_options: u16,
    pub capabilities0: u16,
    pub capabilities1: u16,
    obsolete4: [u16; 2],
    pub other1: u16, // 53
    obsolete5: [u16; 5], // 54..58
    pub other2: u16, // 59
    pub user_addressable_logical_sectors_28: u32, // 60..61
    obsolete6: u16,
    pub other3: u16, // 63
    pub other4: u16, // 64
    pub minimum_multiword_dma_transfer_cycle_time_per_word: u16,
    pub recommended_multiword_dma_transfer_cycle_time: u16,
    pub minimum_pio_transfer_cycle_time_without_flow_control: u16,
    pub minimum_pio_transfer_cycle_time_with_iordy: u16,
    pub additional_supported: u16,
    reserved0: u16,
    reserved_for_identify_packet_device_command: [u16; 4], // 71..74
    pub queue_depth: u16,
    pub sata_capabilities: u16,
    pub sata_additional_capabilities: u16,
    pub sata_features_supported: u16,
    pub sata_features_enabled: u16,
    pub major_version_number: u16,
    pub minor_version_number: u16,
    pub commands_and_feature_sets_supported0: u16,
    pub commands_and_feature_sets_supported1: u16,
    pub commands_and_feature_sets_supported2: u16,
    pub commands_and_feature_sets_supported_or_enabled0: u16,
    pub commands_and_feature_sets_supported_or_enabled1: u16,
    pub commands_and_feature_sets_supported_or_enabled2: u16,
    pub ultra_dma_modes: u16,
    pub other5: u16, // 89
    pub other6: u16, // 90
    reserved1: u8,
    pub current_apm_level_value: u8,
    pub master_password_identifier: u16,
    pub hardware_reset_results: u16,
    obsolete7: u16,
    pub stream_minimum_request_size: u16,
    pub streaming_transfer_time_dma: u16,
    pub streaming_access_latency: u16,
    pub streaming_performance_granularity: u32,
    pub user_addressable_logical_sectors_48: u64,
    pub streaming_transfer_time_pio: u16,
    pub max_number_of_512_byte_blocks_per_data_set_management_command: u16,
    pub other7: u16, // 106
    pub inter_seek_delay_for_iso_iec_7779_standard_acoustic_testing: u16,
    pub world_wide_name: [u16; 4],
    reserved2: [u16; 4],
    obsolete8: u16,
    pub logical_sector_size: u32,
    pub commands_and_feature_sets_supported3: u16,
    pub commands_and_feature_sets_supported_or_enabled3: u16,
    reserved_for_expanded_supported_and_enabled_settings: [u16; 6], // 121..126
    obsolete9: u16,
    pub security_status: u16,
    pub vendor_specific: [u16; 31], // 129..159
    reserved_for_cfa1: [u16; 8], // 160..167
    pub other8: u16, // 168
    pub data_set_management_command_support: u16,
    pub additional_product_identifier: [u16; 4],
    reserved3: [u16; 2],
    pub current_media_serial_number: [u16; 29], // 176..205
    pub sct_command_transport: u16,
    reserved4: [u16; 2],
    pub alignment_of_logical_sectors_within_a_physical_sector: u16,
    pub write_read_verify_sector_mode_3_count: u32,
    pub write_read_verify_sector_mode_2_count: u32,
    obsolete10: [u16; 3],
    pub nominal_media_rotation: u16,
    reserved5: u16,
    obsolete11: u16,
    pub other9: u16, // 220
    reserved6: u16,
    pub transport_major_version_number: u16,
    pub transport_minor_version_number: u16,
    reserved7: [u16; 6],
    pub extended_number_of_user_addressable_sectors: u64,
    pub minimum_number_of_512_byte_data_blocks_per_download_microcode_operation: u16,
    pub maximum_number_of_512_byte_data_blocks_per_download_microcode_operation: u16,
    reserved8: [u16; 19], // 236..254
    pub integrity_word: u16,
}

#[derive(Debug, Clone, Copy)]
struct TaskFileData(u32);

impl TaskFileData {
    fn error(&self) -> Option<u8> {
        let err = ((self.0 >> 0) & 0xFF) as u8;
        if err != 0 {
            return Some(err);
        } else {
            return None;
        }
    }

    fn busy(&self) -> bool {
        self.0 & (1 << 7) != 0
    }

    fn transfer_requested(&self) -> bool {
        self.0 & (1 << 3) != 0
    }

    fn transfer_error(&self) -> bool {
        self.0 & (1 << 0) != 0
    }
}

#[derive(Debug, Clone, Copy)]
struct SataStatus {
    ipm: Option<IpmState>,
    spd: Option<InterfaceSpeed>,
    det: DeviceDetection,
}

impl TryFrom<u32> for SataStatus {
    type Error = u32;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        let ipm_bits = (value >> 8) & 0b1111;
        let spd_bits = (value >> 4) & 0b1111;
        let det_bits = value & 0b1111;
        let ipm = match ipm_bits {
            0 => None,
            1 => Some(IpmState::Active),
            2 => Some(IpmState::Partial),
            6 => Some(IpmState::Slumber),
            8 => Some(IpmState::DevSleep),
            _ => return Err(value),
        };
        let spd = match spd_bits {
            0 => None,
            1 => Some(InterfaceSpeed::Gen1),
            2 => Some(InterfaceSpeed::Gen2),
            3 => Some(InterfaceSpeed::Gen3),
            _ => return Err(value),
        };
        let det = match det_bits {
            0 => DeviceDetection::NoDeviceDetected,
            1 => DeviceDetection::DeviceDetected,
            3 => DeviceDetection::PhyCommunicationEstablished,
            4 => DeviceDetection::PhyOfflineMode,
            _ => return Err(value),
        };

        Ok(Self { ipm, spd, det })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IpmState {
    Active,
    Partial,
    Slumber,
    DevSleep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeviceDetection {
    NoDeviceDetected,
    DeviceDetected,
    PhyCommunicationEstablished,
    PhyOfflineMode,
}

// TODO: create custom Debug/Display impl
#[derive(Debug, Clone, Copy)]
struct SataError(u32);

macro_rules! sata_error{
    {$($flag:ident: $bit:expr),* $(,)?} => {
        #[allow(unused)]
        impl SataError {
            $(
                pub fn $flag(&self) -> bool {
                    self.0 & (1 << $bit) != 0
                }
            )*
        }
    }
}

sata_error! {
    exchanged: 26,
    unknown_fis_type: 25,
    transport_state_transition_error: 24,
    link_sequence_error: 23,
    handshake_error: 22,
    crc_error: 21,
    disparity_error: 20,
    decode_10b_to_8b_error: 19,
    comm_wake: 18,
    phy_internal_error: 17,
    phy_rdy_change: 16,
    internal_error: 11,
    protocol_error: 10,
    persistent_communication_or_data_integrity_error: 9,
    transient_data_integrity_error: 8,
    recovered_communications_error: 1,
    recovered_data_integrity_error: 0,
}

#[allow(unused)]
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum FisType {
    RegisterH2D = 0x27,
    RegisterD2H = 0x34,
    ActivateDma = 0x39,
    SetupDma = 0x41,
    Data = 0x46,
    Bist = 0x58,
    SetupPio = 0x5F,
    DeviceBits = 0xA1,
}

// #[derive(Debug, Clone, Copy)]
// #[repr(C)]
// struct RegisterFisH2D {
//     // DWORD 0
//     ty: FisType,
//     flags: u8,
//     command: u8,
//     feature_low: u8,
//     // DWORD 1
//     lba0: u8,
//     lba1: u8,
//     lba2: u8,
//     device: u8,
//     // DWORD 2
//     lba3: u8,
//     lba4: u8,
//     lba5: u8,
//     feature_high: u8,
//     // DWORD 3
//     count_low: u8,
//     count_high: u8,
//     icc: u8,
//     control: u8,
//     // DWORD 4
//     reserved: [u8; 4],
// }
//
// impl RegisterFisH2D {
//     fn new() -> Self {
//         Self {
//             ty: FisType::RegisterH2D,
//             flags: 0,
//         }
//     }
// }

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

#[repr(C, packed)]
struct CommandTable {
    cfis: CommandFis,
    acmd: AtapiCommand,
    reserved: [u8; 0x30],
    prdts: [Prdt],
}

impl CommandTable {
    fn new(cfis: CommandFis, acmd: AtapiCommand, prdts: &[Prdt]) -> Box<Self> {
        let mut command_table = Self::zero(prdts.len());

        command_table.cfis = cfis;
        command_table.acmd = acmd;
        for (i, prdt) in prdts.iter().enumerate() {
            command_table.prdts[i] = *prdt;
        }

        command_table
    }

    // TODO: check if this is correct
    fn zero(prdt_count: usize) -> Box<Self> {
        let size = mem::size_of::<CommandFis>()
            + mem::size_of::<AtapiCommand>()
            + mem::size_of::<[u8; 0x30]>()
            + mem::size_of::<Prdt>() * prdt_count;
        let layout = Layout::from_size_align(size, 128).unwrap();
        let ptr = unsafe { alloc_zeroed(layout) };
        if ptr.is_null() {
            panic!("failed to allocate CommandTable (got a null-pointer)");
        }
        let ptr: *mut CommandTable = ptr::from_raw_parts_mut(ptr, prdt_count);

        unsafe { Box::from_raw(ptr) }
    }

    fn get_address(&self) -> u64 {
        &*self as *const CommandTable as *const () as usize as u64
    }
}

impl Debug for CommandTable {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "CommandTable {{ cfis: {:?}, acmd: {:?}, prdts: [",
            self.cfis, self.acmd
        )?;
        write!(
            f,
            "{}] }}",
            self.prdts
                .iter()
                .map(|prdt| format!("{:?}", prdt))
                .collect::<Vec<String>>()
                .join(", ")
        )
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct Prdt {
    dba: u32,
    dbau: u32,
    reserved: u32,
    dw3: u32,
}

impl Prdt {
    fn new(address: u64, count: u32) -> Self {
        if address & 0x1 != 0 {
            panic!(
                "tried to create Prdt with address {}; must be word-aligned",
                address
            );
        }
        let dba = address as u32;
        let dbau = (address >> 32) as u32;

        const COUNT_MASK: u32 = 0b11_1111_1111_1111_1111_1111; // 22 bits
        if count & !COUNT_MASK != 0 {
            panic!("tried to create Prdt with size {} (> 4 MiB)", count);
        }
        let dw3 = count;

        Self {
            dba,
            dbau,
            reserved: 0,
            dw3,
        }
    }
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

        let layout =
            Layout::from_size_align(core::mem::size_of::<[CommandHeader; 32]>(), 1024).unwrap();
        let ptr = unsafe { alloc(layout) } as *mut [MaybeUninit<CommandHeader>; 32];
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
#[derive(Debug)]
#[repr(align(256))]
pub struct FisReceiveArea([u8; 256]);

impl FisReceiveArea {
    pub fn new() -> Self {
        Self([0; 256])
    }
}
