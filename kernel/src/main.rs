#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    // End the panic handler in an infinite loop to halt the system
    loop {}
}

pub struct BootInfo;

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main(_boot_info: &'static BootInfo) {
    todo!();
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(boot_info: &'static BootInfo) -> ! {
    kernel_main(boot_info);

    loop {}
}
