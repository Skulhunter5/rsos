use core::ffi::c_void;

use crate::uefi::Status;

#[repr(C)]
pub struct SimpleTextOutputProtocol {
    pub reset: Reset,
    pub output_string: OutputString,
    test_string: *const c_void,
    query_mode: *const c_void,
    set_mode: *const c_void,
    set_attribute: *const c_void,
    clear_screen: *const c_void,
    set_cursor_position: *const c_void,
    enable_cursor: *const c_void,
    mode: *const c_void,
}

pub type OutputString = extern "efiapi" fn(
    output_protocol: *const SimpleTextOutputProtocol,
    string: *const u16,
) -> Status;

pub type Reset = extern "efiapi" fn(
    output_protocol: *const SimpleTextOutputProtocol,
    extended_verification: bool,
) -> Status;
