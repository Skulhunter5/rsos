use crate::PhysicalAddress;

// TODO: fix address mask to include "execute disable" bit etc.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct PageEntry<const LEVEL: usize>(u64);

impl<const LEVEL: usize> PageEntry<LEVEL> {
    pub fn empty() -> Self {
        Self(0)
    }

    pub fn address(&self) -> PhysicalAddress {
        PhysicalAddress::from(self.0 & !0xFFF)
    }

    pub fn set_address(&mut self, address: PhysicalAddress) {
        self.0 = (self.0 & 0xFFF) | (address.0 as u64 & !0xFFF);
    }

    pub fn present(&self) -> bool {
        self.0 & (1 << 0) != 0
    }

    pub fn set_present(&mut self, present: bool) {
        let mask = (present as u64) << 0;
        if present {
            self.0 |= mask;
        } else {
            self.0 &= !mask;
        }
    }

    pub fn writable(&self) -> bool {
        self.0 & (1 << 1) != 0
    }

    pub fn set_writable(&mut self, writable: bool) {
        let mask = (writable as u64) << 1;
        if writable {
            self.0 |= mask;
        } else {
            self.0 &= !mask;
        }
    }

    pub fn user_access(&self) -> bool {
        self.0 & (1 << 2) != 0
    }

    pub fn write_through(&self) -> bool {
        self.0 & (1 << 3) != 0
    }

    pub fn cache_disabled(&self) -> bool {
        self.0 & (1 << 4) != 0
    }

    pub fn cacheable(&self) -> bool {
        self.0 & (1 << 4) == 0
    }

    pub fn set_cacheable(&mut self, cacheable: bool) {
        let mask = (cacheable as u64) << 4;
        if cacheable {
            self.0 &= !mask;
        } else {
            self.0 |= mask;
        }
    }

    pub fn accessed(&self) -> bool {
        self.0 & (1 << 5) != 0
    }

    pub fn dirty(&self) -> bool {
        self.0 & (1 << 6) != 0
    }

    pub fn is_final_page(&self) -> bool {
        self.0 & (1 << 7) != 0
    }

    pub fn page_size(&self) -> Option<usize> {
        if self.is_final_page() {
            Some(4096 * 512usize.pow(LEVEL as u32 - 1))
        } else {
            None
        }
    }

    pub fn set_final_page(&mut self, final_page: bool) {
        let mask = (final_page as u64) << 7;
        if final_page {
            self.0 |= mask;
        } else {
            self.0 &= !mask;
        }
    }

    pub fn execute_disabled(&self) -> bool {
        self.0 & (1 << 63) != 0
    }
}

// TODO: improve/complete this Debug implementation
impl<const LEVEL: usize> core::fmt::Debug for PageEntry<LEVEL> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // const FLAG_MAP: &[(fn(&PageEntry<LEVEL>) -> bool, &str)] = &[
        //     (PageEntry::present, "present"),
        //     (PageEntry::user_access, "user_access"),
        // ];

        if self.0 == 0 {
            return write!(f, "0");
        }
        // write!(
        //     f,
        //     "PageEntry {{ address: {:?}, other: [{}] }}",
        //     self.address(),
        //     Self::FLAG_MAP
        //         .iter()
        //         .filter(|(check, _)| check(self))
        //         .map(|(_, s)| *s)
        //         .collect::<alloc::vec::Vec<_>>()
        //         .join("|")
        // )
        let mut flags = alloc::string::String::with_capacity(8);
        flags.push(if self.present() { 'p' } else { '-' });
        flags.push(if self.writable() { 'w' } else { '-' });
        flags.push(if self.execute_disabled() { '-' } else { 'x' });
        flags.push(if self.user_access() { 'u' } else { 's' });
        flags.push(if self.dirty() { 'd' } else { '-' });
        flags.push(if self.cacheable() { 'c' } else { '-' });
        flags.push(if self.is_final_page() { 'f' } else { '-' });
        write!(f, "PageEntry({:?}, {})", self.address(), flags)
    }
}

