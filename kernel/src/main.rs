#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate alloc;

use core::panic::PanicInfo;

use alloc::{boxed::Box, string::ToString};
use common::{BootInfo, allocation::FixedBufferAllocator, spin::Mutex};
use kernel::{
    gdt::{self, GlobalDescriptorTable, TaskStateSegment},
    idt::InterruptDescriptorTable,
    uart,
};
use memory::PhysicalMemoryManager;

mod interrupts;
mod memory;

// temporary implementation for system shutdown
fn crash_system() {
    gdt::reload_segment_registers(0, 0);
}

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

const HEAP_SIZE: usize = 1024 * 1024;
#[global_allocator]
static GLOBAL_ALLOCATOR: FixedBufferAllocator<HEAP_SIZE> = FixedBufferAllocator::new();

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

static GDT: Mutex<Option<Box<GlobalDescriptorTable>>> = Mutex::new(None);
static TSS: Mutex<Option<Box<TaskStateSegment>>> = Mutex::new(None);
static IDT: Mutex<Option<Box<InterruptDescriptorTable>>> = Mutex::new(None);

#[unsafe(no_mangle)]
pub fn kernel_main(bootinfo: &BootInfo) {
    println!("setting up frame allocator...");
    let mut pmm = PhysicalMemoryManager::init(&bootinfo.available_mem).expect("failed to initialize PhysicalMemoryManager");
    println!("-> done");
    println!("allocating frame...");
    let frame = pmm.alloc_page().expect("failed to allocate frame from PhysicalMemoryManager");
    println!("- frame: 0x{:x}", frame.0);
    pmm.free_page(frame);
    println!("-> done");

    print!("loading gdt...");
    let (gdt, tss) = gdt::init();
    GDT.lock().replace(gdt);
    TSS.lock().replace(tss);
    println!(" done");

    print!("setting up interrupts...");
    let idt = interrupts::init();
    IDT.lock().replace(idt);
    // unsafe { core::arch::asm!("mov cr3, rax", in("rax") 0); }
    println!(" done");

    println!("trying to cause interrupt...");
    unsafe {
        core::arch::asm!(
            "xor rcx, rcx",
            "div rcx",
            out("rcx") _,
        );
    }
    println!("-> failed");

    // for i in 0..10000000 {
    //     core::hint::black_box(i);
    // }

    println!();
    println!();
    println!();
    crate::println!(":: KERNEL DONE");
    crash_system();
}

#[unsafe(no_mangle)]
pub extern "sysv64" fn _start(bootinfo: *const BootInfo) -> ! {
    unsafe {
        uart::init();
    }
    println!("> Greetings from the kernel");
    println!();
    println!();
    println!();
    kernel_main(unsafe { bootinfo.as_ref().unwrap() });

    println!();
    println!();
    println!();
    println!("KERNEL MAIN EXITED");
    loop {}
}
