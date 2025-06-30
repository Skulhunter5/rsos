#![no_std]
#![no_main]
#![feature(allocator_api)]
#![feature(ptr_metadata)]
#![feature(alloc_layout_extra)]

#[cfg(not(target_pointer_width = "64"))]
compile_error!("unsupported target pointer width");

extern crate alloc;

use core::panic::PanicInfo;

use ahci::AhciController;
use alloc::{string::ToString, vec::Vec};
use bootloader::{
    acpi::AcpiTables,
    allocator::LinearAllocator,
    elf::{self, Elf},
    pci::{self, DeviceType, MassStorageControllerType, SataControllerInterface},
};
use disk::{Disk, PartitionDevice, StorageDevice};
use fat::FatFs;
use uefi::{
    SystemTable,
    raw::{self, ImageHandle, MemoryType},
};

mod ahci;
mod disk;
mod fat;
mod uart;
pub mod uefi;
pub mod uefi2;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe {
        uart::puts("!!! PANIC !!!\n");
        match info.message().as_str() {
            Some(message) => {
                uart::puts(message);
                uart::puts("\n");
            }
            None => {
                uart::puts("PanicMessage.as_str() failed\n");
                let message = info.message().to_string();
                uart::puts(&message);
                uart::puts("\n");
            }
        }
    }

    // End the panic handler in an infinite loop to halt the system
    loop {}
}

//struct UefiWriter {
//    con_out: *mut uefi::SimpleTextOutputProtocol,
//}
//
//unsafe impl Send for UefiWriter {}
//
//static UEFI_WRITER: spin::Mutex<Option<UefiWriter>> = spin::Mutex::new(None);
//
//impl Write for UefiWriter {
//    fn write_str(&mut self, s: &str) -> core::fmt::Result {
//        for c in s.chars() {
//            let string_arr = [c as u16, '\0' as u16];
//            let status = unsafe { ((*self.con_out).output_string)(self.con_out, &string_arr[0]) };
//            if status.is_error() {
//                return Err(core::fmt::Error);
//            }
//        }
//        Ok(())
//    }
//}

//fn setup_uefi_writer(system_table: &SystemTable) {
//    let writer = UefiWriter {
//        con_out: system_table.con_out as *mut uefi::SimpleTextOutputProtocol,
//    };
//    let mut guard = UEFI_WRITER.lock();
//    *guard = Some(writer);
//}

#[macro_export]
#[allow(unused)]
macro_rules! print {
    ($($arg:tt)*) => ($crate::uart::_print(format_args!($($arg)*)));
}

#[macro_export]
#[allow(unused)]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[global_allocator]
static GLOBAL_ALLOCATOR: LinearAllocator = LinearAllocator::new();

pub unsafe fn halt() {
    unsafe {
        core::arch::asm!("hlt", options(att_syntax, nomem, nostack),);
    }
}

#[unsafe(no_mangle)]
pub extern "efiapi" fn efi_main(handle: ImageHandle, system_table: *mut raw::tables::SystemTable) {
    unsafe {
        uefi2::init(
            handle as uefi2::raw::Handle,
            system_table as *mut uefi2::raw::SystemTable,
        );
    }

    let system_table = unsafe { system_table.as_mut().expect("UEFI SystemTable is nullptr") };
    let system_table = unsafe { SystemTable::from(system_table) };

    unsafe { uart::init() };

    let mut stdout = system_table.stdout().unwrap();
    stdout.clear().unwrap();
    stdout.puts("Hello from UEFI\n").unwrap();

    println!("Hello from UART");
    println!("Stalling 0.25 seconds...");

    let boot_services = system_table.boot_services().expect("missing boot services");
    boot_services.stall_us(1_000_000 / 4);

    println!("Continuing!");

    let config_table = system_table.config_table().unwrap();
    //for entry in config_table.iter() {
    //    if entry.guid == uefi::Guid::EFI_ACPI_TABLE_GUID {
    //        println!("Found ACPI Table");
    //    }
    //}

    let rsdp = config_table.get_rsdp().unwrap();
    println!("RSDP revision: {}", rsdp.revision());
    for table in AcpiTables::from_rsdp(rsdp).unwrap() {
        println!("{:?}", table);
    }

    let mut storage_device = None;
    for device in unsafe { pci::enumerate() } {
        match device.device_type().unwrap() {
            DeviceType::MassStorageController(MassStorageControllerType::SataController {
                interface: SataControllerInterface::Ahci,
            }) => {
                storage_device = Some(device);
                break;
            }
            _ => {}
        }
    }
    let storage_device =
        storage_device.expect("failed to find ahci controller during pci enumeration");

    let mut ahci_controller =
        AhciController::try_from(storage_device).expect("failed to create AhciController");

    let port = ahci_controller.get_port(0).unwrap();
    let partitions = Disk::read_partitions(port).unwrap();
    if partitions.len() == 0 {
        panic!("No partitions found");
    }
    let partition = partitions[0];

    let mut partition = PartitionDevice::new(port, partition);

    let mut fs = FatFs::wrap(&mut partition as &mut dyn StorageDevice).unwrap();
    let files = fs.list_directory("/EFI/BOOT").unwrap();
    println!("Files: {:?}", &files);
    let kernel = fs.read_file("/EFI/BOOT/KERNEL").unwrap();
    println!("Kernel size: {} bytes", kernel.len());

    println!();
    let elf = Elf::parse(&kernel).unwrap();
    // println!("Elf: {:x?}", elf);

    if let Some(_) = elf
        .section_headers
        .iter()
        .position(|header| header.ty == elf::SectionType::UninitializedSpace)
    {
        todo!("loading elf section .bss");
    }

    let sections = elf
        .section_headers
        .iter()
        .filter(|header| header.flags.allocated())
        .collect::<Vec<_>>();
    println!("Important sections:\n{:x?}", sections);

    drop(stdout);
    drop(boot_services);
    drop(system_table);

    let memory_map = unsafe { uefi2::boot::exit_boot_services() };

    let usable_memory = memory_map
        .iter()
        .filter(|entry| entry.ty == uefi2::raw::MemoryType::CONVENTIONAL_MEMORY)
        .map(|entry| {
            (
                entry.physical_start,
                entry.physical_start + entry.page_count * 4096,
            )
        })
        .collect::<Vec<_>>();
    let total_usable_memory = usable_memory
        .iter()
        .map(|(start, end)| end - start)
        .sum::<u64>();
    println!(
        "usable memory: {:x?} (0x{:x} bytes total)",
        &usable_memory, total_usable_memory
    );
    let reclaimable_memory = memory_map
        .iter()
        .filter(|entry| {
            entry.ty == uefi2::raw::MemoryType::BOOT_SERVICES_CODE
                || entry.ty == uefi2::raw::MemoryType::BOOT_SERVICES_DATA
                || entry.ty == uefi2::raw::MemoryType::LOADER_CODE
                || entry.ty == uefi2::raw::MemoryType::LOADER_DATA
        })
        .map(|entry| {
            (
                entry.physical_start,
                entry.physical_start + entry.page_count * 4096,
            )
        })
        .fold(Vec::<(u64, u64)>::new(), |mut list, entry| {
            if let Some(last_entry) = list.last_mut() {
                if last_entry.1 == entry.0 {
                    last_entry.1 = entry.1;
                    list
                } else {
                    list.push(entry);
                    list
                }
            } else {
                list.push(entry);
                list
            }
        });
    let total_reclaimable_memory = reclaimable_memory
        .iter()
        .map(|(start, end)| end - start)
        .sum::<u64>();
    println!(
        "reclaimable memory: {:x?} (0x{:x} bytes total)",
        &reclaimable_memory, total_reclaimable_memory
    );

    println!("\n\nDONE -> LOOPING...");
    loop {}
}
