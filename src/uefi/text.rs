use core::{fmt::Write, sync::atomic::Ordering};

use super::{RawSystemTable, SimpleTextOutputProtocol};

pub struct Output<'a> {
    table: &'a RawSystemTable,
    proto: &'a mut SimpleTextOutputProtocol,
}

impl<'a> Output<'a> {
    pub fn new(table: &'a RawSystemTable, proto: &'a mut SimpleTextOutputProtocol) -> Self {
        Self { table, proto }
    }

    pub fn clear(&mut self) -> Result<(), ()> {
        let status = (self.proto.reset)(self.proto, false);
        if status.is_error() {
            return Err(());
        } else {
            return Ok(());
        }
    }

    pub fn puts(&mut self, s: &str) -> Result<(), ()> {
        const STRING_BUF_LEN: usize = 16;
        let mut string_arr = [0u16; STRING_BUF_LEN + 1];
        let mut i = 0;
        for c in s.chars() {
            string_arr[i] = c as u16;
            i += 1;
            if i == STRING_BUF_LEN {
                i = 0;
                let status = (self.proto.output_string)(self.proto, &string_arr[0]);
                if status.is_error() {
                    return Err(());
                }
            }
        }
        if i > 0 {
            string_arr[i] = '\0' as u16;
            let status = (self.proto.output_string)(self.proto, &string_arr[0]);
            if status.is_error() {
                return Err(());
            }
        }

        Ok(())
    }
}

impl Write for Output<'_> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        match self.puts(s) {
            Ok(()) => Ok(()),
            Err(()) => Err(core::fmt::Error),
        }
    }
}

impl Drop for Output<'_> {
    fn drop(&mut self) {
        self.table.con_out.swap(self.proto, Ordering::Release);
    }
}
