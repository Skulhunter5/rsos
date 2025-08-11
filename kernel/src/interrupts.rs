use kernel::idt::{IdtEntry, InterruptDescriptorTable};

static MSG_DE: &str = "EXCEPTION #DE\0";

#[unsafe(naked)]
unsafe extern "C" fn raw_isr_de() {
    core::arch::naked_asm!(
        "push rax",
        "push rbx",
        "push rcx",
        "push rdx",
        "push rbp",
        "push rsi",
        "push rdi",
        "push r8",
        "push r9",
        "push r10",
        "push r11",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        "mov rdi, [{}]",
        "call {}",
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop r11",
        "pop r10",
        "pop r9",
        "pop r8",
        "pop rdi",
        "pop rsi",
        "pop rbp",
        "pop rdx",
        "pop rcx",
        "pop rbx",
        "pop rax",
        "iretq",
        sym MSG_DE,
        sym put_s,
    );
}

extern "sysv64" fn put_s(ptr: *const core::ffi::c_char) {
    let s = unsafe { core::ffi::CStr::from_ptr(ptr) };
    let s = s.to_str().unwrap();
    crate::println!("{}", s);
}

pub fn init() {
    let mut idt = InterruptDescriptorTable::zero();
    idt[0] = IdtEntry::new(raw_isr_de as *const fn() as u64, 0x08, 0, attributes)
}
