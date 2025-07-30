use core::arch::asm;

#[inline]
pub unsafe fn inb(port: u16) -> u8 {
    let mut value;
    unsafe {
        asm!(
            "in al, dx",
            in("dx") port,
            out("al") value,
        );
    }

    value
}

#[inline]
pub unsafe fn inw(port: u16) -> u16 {
    let mut value;
    unsafe {
        asm!(
            "in ax, dx",
            in("dx") port,
            out("ax") value,
        );
    }

    value
}

#[inline]
pub unsafe fn inl(port: u16) -> u32 {
    let mut value;
    unsafe {
        asm!(
            "in eax, dx",
            in("dx") port,
            out("eax") value,
        );
    }

    value
}

#[inline]
pub unsafe fn outb(port: u16, value: u8) {
    unsafe {
        asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
        );
    }
}

#[inline]
pub unsafe fn outl(port: u16, value: u32) {
    unsafe {
        asm!(
            "out dx, eax",
            in("dx") port,
            in("eax") value,
        );
    }
}
