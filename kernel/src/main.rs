#![no_std]
#![no_main]

extern crate alloc;

use core::{alloc::GlobalAlloc, panic::PanicInfo, ptr};

use alloc::string::ToString;

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

#[global_allocator]
static KERNEL_ALLOCATOR: KernelAllocator = KernelAllocator;

struct KernelAllocator;

unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, _layout: core::alloc::Layout) -> *mut u8 {
        ptr::null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {}
}

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
    return 123456;
}
