#![no_std]
#![no_main]
#![feature(allocator_api)]
#![feature(inline_const_pat)]

extern crate alloc;

use core::{alloc::GlobalAlloc, panic::PanicInfo};

use alloc::alloc::Global;
use uefi::{
    SystemTable,
    raw::{self, ImageHandle},
};

mod io;
mod pci;
mod spin;
mod uart;
pub mod uefi;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe {
        uart::puts("!!! PANIC !!!");
        match info.message().as_str() {
            Some(message) => uart::puts(message),
            None => uart::puts("PanicMessage.as_str() failed"),
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
static GLOBAL_ALLOCATOR: BootloaderAllocator = BootloaderAllocator;

const HEAP_SIZE: usize = 64 * 1024;
static mut HEAP: [u8; HEAP_SIZE] = [0u8; HEAP_SIZE];
#[allow(static_mut_refs)]
static mut HEAP_PTR: *const u8 = unsafe { HEAP.as_ptr() };
#[allow(static_mut_refs)]
const HEAP_END: *const u8 = unsafe { HEAP.as_ptr().byte_add(HEAP_SIZE) };

struct BootloaderAllocator;

// TODO: make the global allocator threadsafe
unsafe impl GlobalAlloc for BootloaderAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        if unsafe { HEAP_PTR } >= HEAP_END {
            return 0 as *mut u8;
        }

        let ptr = unsafe { HEAP_PTR };
        let off = layout.size() - (ptr as usize % layout.align());
        let aligned_ptr = unsafe { ptr.byte_add(off) };
        unsafe {
            HEAP_PTR = aligned_ptr.byte_add(layout.size());
        }

        if unsafe { HEAP_PTR.byte_sub(1) } >= HEAP_END {
            return 0 as *mut u8;
        }

        return aligned_ptr.cast_mut();
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {}
}

pub unsafe fn halt() {
    unsafe {
        core::arch::asm!("hlt", options(att_syntax, nomem, nostack),);
    }
}

#[unsafe(no_mangle)]
pub extern "efiapi" fn efi_main(_handle: ImageHandle, system_table: *mut raw::tables::SystemTable) {
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

    let memory_map_size = boot_services
        .get_memory_map_size()
        .expect("failed to get memory map");
    println!("mem-map size: {}", memory_map_size);
    let memory_map = boot_services.get_memory_map(Global).unwrap();
    println!("memory_map: {:?}", memory_map);
    //for entry in memory_map.iter() {
    //    println!("- {:?}", entry);
    //}
    //for i in 0..memory_map.len() {
    //    println!("- {}: {:?}", i, memory_map.get(i));
    //}

    for entry in system_table.config_table().unwrap().iter() {
        if entry.guid == uefi::Guid::EFI_ACPI_TABLE_GUID {
            println!("Found ACPI Table");
        }
    }

    println!();
    println!("PCI:");
    unsafe {
        pci::test();
    }
    //println!("- VendorId: {:#x}", Pci::get_vendor_id(0, 0, 0));

    loop {}
}
