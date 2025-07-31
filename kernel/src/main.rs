#![no_std]
#![no_main]

extern crate alloc;

use core::panic::PanicInfo;

use alloc::string::ToString;
use common::allocation::FixedBufferAllocator;

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

pub struct BootInfo;

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main(_bootinfo: &'static BootInfo) {
    todo!();
}

// #[unsafe(no_mangle)]
// pub extern "C" fn _start(bootinfo: *const BootInfo) -> ! {
//     unsafe { uart::init() };
//     println!("> Greetings from the kernel");
//     println!("  > bootinfo: {:?}", bootinfo);
//     kernel_main(unsafe { bootinfo.as_ref().unwrap() });
//
//     loop {}
// }

#[unsafe(no_mangle)]
pub extern "C" fn _start(_bootinfo: *const BootInfo) -> u64 {
    unsafe {
        uart::init();
    }
    println!("> Greetings from the kernel");
    return 123456;
}
