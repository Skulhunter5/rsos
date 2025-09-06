#![no_std]
#![no_main]
#![feature(allocator_api)]
#![feature(ptr_metadata)]
#![feature(alloc_layout_extra)]

#[cfg(not(target_pointer_width = "64"))]
compile_error!("unsupported target pointer width");
#[cfg(not(target_arch = "x86_64"))]
compile_error!("unsupported target pointer width");

extern crate alloc;

use core::{panic::PanicInfo, ptr};

use acpi::AcpiTables;
use ahci::AhciController;
use alloc::{string::ToString, vec::Vec};
use allocators::{BootloaderPageAllocator, RuntimePageAllocator, UefiPageAllocator};
use bootloader::{
    elf::{Elf, SectionType},
    pci::{self, DeviceType, MassStorageControllerType, SataControllerInterface},
    uefi2,
};
use common::{allocation::FixedBufferAllocator, BootInfo, PhysicalAddress, VirtualAddress};
use disk::{Disk, PartitionDevice, StorageDevice};
use fat::FatFs;
use paging::{
    PageDirectory, PageDirectoryPointer, PageEntry, PageMapLevel4, PageTable, read_cr3, write_cr3,
};
use uefi::{
    SystemTable,
    raw::{self, ImageHandle},
};

mod ahci;
mod allocators;
mod disk;
mod fat;
mod paging;
mod uart;
pub mod uefi;

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

// Global Allocator
// - backed by current Page Allocator
// - maybe just do a bunch of LinearAllocators, one per area of free space without frees
//   -> fast and simple
//   -> memory can easily be reclaimed in kernel (at least after copying bootinfo)

const HEAP_SIZE: usize = 16 * 1024 * 1024;
#[global_allocator]
static GLOBAL_ALLOCATOR: FixedBufferAllocator<HEAP_SIZE> = FixedBufferAllocator::new();

static PAGE_ALLOCATOR: BootloaderPageAllocator = BootloaderPageAllocator::new();
static UEFI_PAGE_ALLOCATOR: UefiPageAllocator = UefiPageAllocator;
static RUNTIME_PAGE_ALLOCATOR: RuntimePageAllocator = RuntimePageAllocator::new();

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

pub unsafe fn halt() {
    unsafe {
        core::arch::asm!("hlt", options(att_syntax, nomem, nostack),);
    }
}

#[unsafe(no_mangle)]
pub extern "efiapi" fn efi_main(handle: ImageHandle, system_table: *mut raw::tables::SystemTable) {
    uefi2::init(
        handle as uefi2::raw::Handle,
        system_table as *mut uefi2::raw::SystemTable,
    );

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

    println!();
    println!();
    println!();

    print!("reading kernel from FAT filesystem...");
    let mut fs = FatFs::wrap(&mut partition as &mut dyn StorageDevice).unwrap();
    // let files = fs.list_directory("/EFI/BOOT").unwrap();
    // println!("Files: {:?}", &files);
    let kernel = fs.read_file("/EFI/BOOT/KERNEL").unwrap();
    println!(" done (size: {} bytes)", kernel.len());

    print!("parsing kernel elf file...");
    let elf = Elf::parse(&kernel).unwrap();
    println!(" done");
    assert!(elf.entry != 0);

    let sections = elf
        .section_headers
        .iter()
        .filter(|header| header.flags.allocated())
        .collect::<Vec<_>>();

    print!("copying runtime sections from file to memory...");
    let kernel_entry_point = elf.entry;
    let section_allocations = sections
        .iter()
        .map(|section| {
            assert!(section.vaddr % 4096 == 0);
            assert!(section.addralign == 4096);

            let pages =
                (section.size / 4096 + if section.size % 4096 > 0 { 1 } else { 0 }) as usize;
            let ptr = uefi2::boot::allocate_pages(
                uefi2::raw::AllocateType::AllocateAnyPages,
                uefi2::raw::MemoryType::LOADER_DATA,
                pages,
                0,
            )
            .expect("failed to allocate pages for section") as *mut u8;
            // TODO: confirm that this is working as intended with .bss section
            if section.ty != SectionType::UninitializedSpace {
                unsafe {
                    let src = kernel.as_ptr().byte_add(section.offset as usize);
                    ptr.copy_from_nonoverlapping(src, section.size as usize);
                }
            } else {
                unsafe {
                    ptr.write_bytes(0, section.size as usize);
                }
            }
            (ptr, pages, section.vaddr)
        })
        .collect::<Vec<_>>();
    println!(" done");

    drop(stdout);
    drop(boot_services);
    drop(system_table);

    let memory_map = unsafe { uefi2::boot::exit_boot_services() };
    PAGE_ALLOCATOR.set_runtime();

    println!();
    println!("exiting boot services...");
    let free_memory = memory_map
        .iter()
        .filter(|entry| entry.ty == uefi2::raw::MemoryType::CONVENTIONAL_MEMORY)
        .map(|entry| {
            (
                entry.physical_start,
                entry.physical_start + entry.page_count * 4096,
            )
        })
        .collect::<Vec<_>>();
    let _total_usable_memory = free_memory
        .iter()
        .map(|(start, end)| end - start)
        .sum::<u64>();
    // println!(
    //     "usable memory: {:x?} (0x{:x} bytes total)",
    //     &free_memory, total_usable_memory
    // );
    RUNTIME_PAGE_ALLOCATOR.setup(&free_memory);
    println!("> success");
    println!();

    let _reclaimable_memory = memory_map
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
    let _total_reclaimable_memory = _reclaimable_memory
        .iter()
        .map(|(start, end)| end - start)
        .sum::<u64>();
    // println!(
    //     "reclaimable memory: {:x?} (0x{:x} bytes total)",
    //     &reclaimable_memory, total_reclaimable_memory
    // );

    {
        print!("creating new paging structures...");
        let pml4 = PageMapLevel4::new_in(&PAGE_ALLOCATOR);
        let pml4_address = PhysicalAddress(ptr::from_ref(pml4) as u64);
        let mut entry_template = PageEntry::empty();
        entry_template.set_present(true);
        entry_template.set_writable(true);
        entry_template.set_cacheable(true);
        for i in 0..2 {
            let pdp = PageDirectoryPointer::new_in(&PAGE_ALLOCATOR);
            let pdp_address = PhysicalAddress(ptr::from_ref(pdp) as u64);

            let mut pe = entry_template.clone();
            pe.set_address(pdp_address);
            pml4.set(i as usize, pe);

            let mut pdp_entry_template = entry_template.clone();
            pdp_entry_template.set_page_size(true);
            for j in 0..512 {
                let page_address = PhysicalAddress((512 * i + j) * (1024 * 1024 * 1024));
                let mut pe = pdp_entry_template.clone();
                pe.set_address(page_address);
                pdp.set(j as usize, pe);
            }
        }
        println!(" done");

        print!("activating new paging structures...");
        let mut cr3_value = read_cr3();
        cr3_value.set_pml4_address(pml4_address);
        write_cr3(cr3_value);
        println!(" done");
    }

    {
        print!("mapping sections to higher half...");
        let phys_to_virt = |paddr: PhysicalAddress| VirtualAddress(paddr.0);
        let mut entry_template = PageEntry::empty();
        entry_template.set_present(true);
        entry_template.set_writable(true);
        entry_template.set_cacheable(true);

        for (section_ptr, pages, vaddr) in section_allocations {
            // println!(
            //     "mapping section: ({:?}, {}, 0x{:x})",
            //     section_ptr, pages, vaddr
            // );
            let start_paddr = section_ptr as usize;
            let start_vaddr = vaddr as usize;
            for i in 0..pages {
                let offset = i * 4096;
                let paddr = start_paddr + offset;
                let vaddr = start_vaddr + offset;

                let pml4 = unsafe { PageMapLevel4::get_current() };
                let pml4_index = (vaddr >> 39) & 0x1FF;
                // println!("> pml4[{}]", pml4_index);

                let pdp = if pml4.is_present(pml4_index) {
                    pml4.next_level(pml4_index, phys_to_virt)
                } else {
                    let pdp = PageDirectoryPointer::new_in(&PAGE_ALLOCATOR);
                    let pdp_address = PhysicalAddress(ptr::from_ref(pdp) as u64);
                    let mut pml4_entry = entry_template.clone();
                    pml4_entry.set_address(pdp_address);
                    pml4.set(pml4_index, pml4_entry);

                    pdp
                };
                let pdp_index = (vaddr >> 30) & 0x1FF;
                // println!("> pdp[{}]", pdp_index);

                let pd = if pdp.is_present(pdp_index) {
                    pdp.next_level(pdp_index, phys_to_virt)
                } else {
                    let pd = PageDirectory::new_in(&PAGE_ALLOCATOR);
                    let pd_address = PhysicalAddress(ptr::from_ref(pd) as u64);
                    let mut pdp_entry = entry_template.clone();
                    pdp_entry.set_address(pd_address);
                    pdp.set(pdp_index, pdp_entry);

                    pd
                };
                let pd_index = (vaddr >> 21) & 0x1FF;
                // println!("> pd[{}]", pd_index);

                let pt = if pd.is_present(pd_index) {
                    pd.next_level(pd_index, phys_to_virt)
                } else {
                    let pt = PageTable::new_in(&PAGE_ALLOCATOR);
                    let pt_address = PhysicalAddress(ptr::from_ref(pt) as u64);
                    let mut pd_entry = entry_template.clone();
                    pd_entry.set_address(pt_address);
                    pd.set(pd_index, pd_entry);

                    pt
                };
                let pt_index = (vaddr >> 12) & 0x1FF;
                // println!("> pt[{}]", pt_index);

                if pt.is_present(pt_index) {
                    panic!(
                        "Trying to map an already present page while mapping higher-half kernel"
                    );
                } else {
                    let mut pt_entry = entry_template.clone();
                    pt_entry.set_address(PhysicalAddress(paddr as u64));
                    pt.set(pt_index, pt_entry);
                }
            }
        }
        println!(" done");
    }

    // let kernel_entry_ptr = kernel_entry_point as *const ();
    // let kernel_entry: unsafe extern "sysv64" fn(bootinfo: *const ()) -> u64 =
    //     unsafe { core::mem::transmute(kernel_entry_ptr) };
    // println!();
    // println!("calling into kernel...");
    // let result = unsafe { kernel_entry(ptr::null()) };
    // println!("> result {}", result);
    // println!("\n\nDONE -> LOOPING...");
    // loop {}

    let bootinfo = BootInfo {
        available_mem: free_memory
            .iter()
            .map(|range| PhysicalAddress(range.0)..PhysicalAddress(range.1))
            .collect::<Vec<_>>(),
    };

    let kernel_entry_ptr = kernel_entry_point as *const ();
    let kernel_entry: extern "sysv64" fn(bootinfo: *const BootInfo) -> ! =
        unsafe { core::mem::transmute(kernel_entry_ptr) };
    kernel_entry(ptr::from_ref(&bootinfo));

    #[allow(unreachable_code)]
    {
        panic!("kernel main function returned");
    }
}
