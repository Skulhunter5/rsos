use core::fmt::Write;

use crate::io::{inb, outb};
use common::spin::Mutex;

pub unsafe fn init() {
    unsafe {
        // DLAB = 1
        outb(0x3FB, 0x80);
        // Divisor = 1 -> baud rate = 115200
        outb(0x3F8, 0x01);
        outb(0x3F9, 0x00);
        // configure 8N1
        outb(0x3FB, 0x03);
        // Enable FIFO and clear buffers
        outb(0x3FA, 0xC7);
    }
}

pub unsafe fn puts(s: &str) {
    for c in s.chars() {
        unsafe {
            putc(c);
        }
    }
}

pub unsafe fn putc(c: char) {
    unsafe {
        while inb(0x3FD) & 0x20 == 0 {}
        outb(0x3F8, c as u8);
    }
}

struct UartWriter;

static UART_WRITER: Mutex<UartWriter> = Mutex::new(UartWriter);

impl Write for UartWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        unsafe {
            puts(s);
        }
        Ok(())
    }
}

pub fn _print(args: core::fmt::Arguments) {
    let mut writer = UART_WRITER.lock();
    let writer = &mut *writer;
    writer.write_fmt(args).unwrap();
}
