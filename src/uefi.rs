#![allow(unused)]

use core::{
    ffi::c_void,
    sync::atomic::{AtomicPtr, Ordering},
};

pub type EfiHandle = *const c_void;
pub type ImageHandle = *const c_void;

type EfiVoidPointer = *const c_void;

pub type EfiSimpleTextInputProtocol = *const c_void;

mod text;

mod guid;
pub use guid::Guid;

mod status;
pub use status::Status;

mod config_table;
pub use config_table::{ConfigurationTable, ConfigurationTableEntry, ConfigurationTableIterator};

mod boot_services;
pub use boot_services::BootServices;

pub struct SystemTable {
    table: &'static RawSystemTable,
}

impl SystemTable {
    pub unsafe fn from(table: &'static RawSystemTable) -> Self {
        Self { table }
    }

    pub fn stdout(&self) -> Option<text::Output> {
        let proto = unsafe {
            self.table
                .con_out
                .swap(core::ptr::null_mut(), Ordering::Acquire)
                .as_mut()?
        };

        Some(text::Output::new(self.table.into(), proto))
    }

    pub fn stderr(&self) -> Option<text::Output> {
        let proto = unsafe {
            self.table
                .std_err
                .swap(core::ptr::null_mut(), Ordering::Acquire)
                .as_mut()?
        };

        Some(text::Output::new(self.table.into(), proto))
    }

    pub fn config_table<'a>(&'a self) -> Option<ConfigurationTable<'a>> {
        if self.table.configuration_table.is_null() {
            None
        } else {
            Some(ConfigurationTable::new(
                self.table.number_of_table_entries,
                self.table.configuration_table,
            ))
        }
    }
}

#[repr(C)]
pub struct TableHeader {
    signature: u64,
    revision: u32,
    size: u32,
    crc32: u32,
    reserved: u32,
}

#[repr(C)]
pub struct RawSystemTable {
    header: TableHeader,
    firmware_vendor: EfiVoidPointer,
    firmware_revision: u32,
    console_in_handle: EfiHandle,
    con_in: EfiSimpleTextInputProtocol,
    console_out_handle: EfiHandle,
    con_out: AtomicPtr<SimpleTextOutputProtocol>,
    standard_error_handle: EfiHandle,
    std_err: AtomicPtr<SimpleTextOutputProtocol>,
    runtime_services: EfiVoidPointer,
    boot_services: AtomicPtr<BootServices>,
    number_of_table_entries: usize,
    configuration_table: *const ConfigurationTableEntry,
}

#[repr(C)]
pub struct SimpleTextOutputProtocol {
    pub reset: Reset,
    pub output_string: OutputString,
    test_string: EfiVoidPointer,
    query_mode: EfiVoidPointer,
    set_mode: EfiVoidPointer,
    set_attribute: EfiVoidPointer,
    clear_screen: EfiVoidPointer,
    set_cursor_position: EfiVoidPointer,
    enable_cursor: EfiVoidPointer,
    mode: EfiVoidPointer,
}

pub type OutputString = extern "efiapi" fn(
    output_protocol: *const SimpleTextOutputProtocol,
    string: *const u16,
) -> Status;

pub type Reset = extern "efiapi" fn(
    output_protocol: *const SimpleTextOutputProtocol,
    extended_verification: bool,
) -> Status;
