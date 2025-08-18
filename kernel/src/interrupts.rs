use alloc::boxed::Box;
use kernel::idt::{IdtEntry, InterruptDescriptorTable};

static MSG_DE: &str = "EXCEPTION #DE\0";

type VirtualAddress = u64;
type SegmentSelector = u16;
type RFlags = u64;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct InterruptStackFrame {
    instruction_pointer: VirtualAddress,
    code_segment: SegmentSelector,
    _pad0: [u8; 6],
    cpu_flags: RFlags,
    stack_pointer: VirtualAddress,
    stack_segment: SegmentSelector,
    _pad1: [u8; 6],
}

macro_rules! make_isr(
    ($idt:ident, $index:expr, $name:ident, $message:expr) => {
        extern "x86-interrupt" fn $name(_stack_frame: InterruptStackFrame) {
            crate::println!("EXCEPTION {}", $message);
        }
        $idt[$index] = IdtEntry::new($name as *const fn() as u64, 0x08, 0, 0x8E);
    }
);

macro_rules! make_isr_err(
    ($idt:ident, $index:expr, $name:ident, $message:expr) => {
        extern "x86-interrupt" fn $name(_stack_frame: InterruptStackFrame, error_code: u64) {
            crate::println!("EXCEPTION {}: {}", $message, error_code);
        }
        $idt[$index] = IdtEntry::new($name as *const fn() as u64, 0x08, 0, 0x8E);
    }
);

// extern "x86-interrupt" fn isr_de(stack_frame: InterruptStackFrame) {
//     crate::println!("EXCEPTION #DE");
// }
//
// extern "x86-interrupt" fn isr_db(stack_frame: InterruptStackFrame) {
//     crate::println!("EXCEPTION #DB");
// }

pub fn init() -> Box<InterruptDescriptorTable> {
    let mut idt = Box::new(InterruptDescriptorTable::zero());
    make_isr!(idt, 0, isr_de, "#DE");
    make_isr!(idt, 1, isr_db, "#DB");
    make_isr!(idt, 2, isr_nmi, "NMI");
    make_isr!(idt, 3, isr_bp, "#BP");
    make_isr!(idt, 4, isr_of, "#OF");
    make_isr!(idt, 5, isr_br, "#BR");
    make_isr!(idt, 6, isr_ud, "#UD");
    make_isr!(idt, 7, isr_nm, "#NM");
    make_isr_err!(idt, 8, isr_df, "#DF");
    make_isr!(idt, 9, isr_cso, "CoprocessorSegmentOverrun");
    make_isr_err!(idt, 10, isr_ts, "#TS");
    make_isr_err!(idt, 11, isr_np, "#NP");
    make_isr_err!(idt, 12, isr_ss, "#SS");
    make_isr_err!(idt, 13, isr_gp, "#GP");
    make_isr_err!(idt, 14, isr_pf, "#PF");
    // make_isr!(idt, 15, isr_intel_reserved, "Intel Reserved");
    make_isr!(idt, 16, isr_mf, "#MF");
    make_isr_err!(idt, 17, isr_ac, "#AC");
    make_isr!(idt, 18, isr_mc, "#MC");
    make_isr!(idt, 19, isr_xm, "#XM");
    make_isr!(idt, 20, isr_ve, "#VE");
    make_isr_err!(idt, 21, isr_cp, "#CP");

    // idt[0] = IdtEntry::new(isr_de as *const fn() as u64, 0x08, 0, 0x8E);
    // idt[1] = IdtEntry::new(isr_de as *const fn() as u64, 0x08, 0, 0x8E);

    idt.load();

    idt
}
