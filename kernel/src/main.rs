#![no_std]
#![no_main]

extern crate alloc;

use core::panic::PanicInfo;

use alloc::string::ToString;
use common::{allocation::FixedBufferAllocator, spin::Mutex, BootInfo};
use kernel::gdt::{self, Gdt};

mod io;
mod uart;

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

static GDT: Mutex<Option<Gdt>> = Mutex::new(None);

#[unsafe(no_mangle)]
pub fn kernel_main(_bootinfo: &BootInfo) {
    unsafe {
        core::arch::asm!(
            "mov ax, 0",
            "mov ds, ax",
            out("ax") _,
        );
    }
    print!("loading gdt...");
    let gdt = gdt::init();
    GDT.lock().replace(gdt);
    GDT.lock();
    println!(" done");
    todo!();
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

    loop {}
}
