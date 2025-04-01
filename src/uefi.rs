use core::sync::atomic::{AtomicBool, Ordering};

pub mod raw;

mod text;

mod guid;
pub use guid::Guid;

mod status;
pub use status::Status;

mod config_table;
pub use config_table::ConfigurationTable;

mod boot_services;
pub use boot_services::BootServices;

mod memory_map;
pub use memory_map::MemoryMap;

pub struct SystemTable {
    table: &'static raw::tables::SystemTable,
    boot_services_lock: AtomicBool,
}

impl SystemTable {
    pub unsafe fn from(table: &'static raw::tables::SystemTable) -> Self {
        Self {
            table,
            boot_services_lock: AtomicBool::new(false),
        }
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

    pub fn boot_services(&self) -> Option<BootServices> {
        if self
            .boot_services_lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(unsafe { BootServices::new(self) })
        } else {
            None
        }
    }
}
