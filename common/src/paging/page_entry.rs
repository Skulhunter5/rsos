use core::marker::PhantomData;

use crate::{PhysicalAddress, VirtualAddress};
use super::{IPageMapLevel, pml, Page4K, Page2M, Page1G, TableOrPage, PageTable, PageDirectory, PageDirectoryPointerTable, PageMapLevel4};

const PAGE_SIZE: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PageOptions(u64);

impl PageOptions {
    const MASK: u64 = 0b11_1111 | (1 << 63);
}

impl Default for PageOptions {
    fn default() -> Self {
        Self(0)
    }
}

pub type PageTableEntry = PageEntry<pml::Pt>;
pub type PageDirectoryEntry = PageEntry<pml::Pd>;
pub type PageDirectoryPointerTableEntry = PageEntry<pml::Pdpt>;
pub type PageMapLevel4Entry = PageEntry<pml::Pml4>;
pub type PageMapLevel5Entry = PageEntry<pml::Pml5>;

pub type PtEntry = PageEntry<pml::Pt>;
pub type PdEntry = PageEntry<pml::Pd>;
pub type PdptEntry = PageEntry<pml::Pdpt>;
pub type Pml4Entry = PageEntry<pml::Pml4>;
pub type Pml5Entry = PageEntry<pml::Pml5>;

#[repr(transparent)]
pub struct PageEntry<L: IPageMapLevel>(u64, PhantomData<L>);

// TODO: add support for the PAT-bit of PageTableEntry
impl<L: IPageMapLevel> PageEntry<L> {
    pub const EMPTY: Self = Self(0, PhantomData);

    const MAXIMUM_PHYSICAL_ADDRESS_BIT: usize = 52;
    const BIT_PRESENT: u64 = 1;
    const BIT_READ_WRITE: u64 = 1 << 1;
    const BIT_USER_SUPERVISOR: u64 = 1 << 2;
    const BIT_WRITE_THROUGH: u64 = 1 << 3;
    const BIT_CACHE_DISABLE: u64 = 1 << 4;
    const BIT_ACCESSED: u64 = 1 << 5;
    const BIT_DIRTY: u64 = 1 << 6;
    const BIT_PAGE_SIZE: u64 = 1 << 7;
    const ADDRESS_MASK: u64 = ((1 << Self::MAXIMUM_PHYSICAL_ADDRESS_BIT) - 1) & !0xFFF;
    const OPTIONS_MASK: u64 = PageOptions::MASK;

    fn new_with_options(options: PageOptions) -> Self {
        Self(options.0, PhantomData)
    }

    pub unsafe fn new_present(paddr: PhysicalAddress, options: PageOptions) -> Self {
        let mut s = Self::new_with_options(options);
        s.set_present(true);
        s.set_address(paddr);
        return s;
    }

    pub fn options(&self) -> PageOptions {
        PageOptions(self.0 & Self::OPTIONS_MASK)
    }

    pub fn set_options(&mut self, options: PageOptions) -> PageOptions {
        let previous = self.options();
        self.0 = (self.0 & Self::ADDRESS_MASK) | options.0;
        return previous;
    }

    pub fn address(&self) -> PhysicalAddress {
        PhysicalAddress::from(self.0 & !0xFFF)
    }

    pub fn set_address(&mut self, address: PhysicalAddress) {
        self.0 = (self.0 & 0xFFF) | (address.0 as u64 & !0xFFF);
    }

    pub fn is_present(&self) -> bool {
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

    pub fn is_page(&self) -> bool {
        self.0 & (1 << 7) != 0
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
impl<L: IPageMapLevel> core::fmt::Debug for PageEntry<L> {
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
        flags.push(if self.is_present() { 'p' } else { '-' });
        flags.push(if self.writable() { 'w' } else { '-' });
        flags.push(if self.execute_disabled() { '-' } else { 'x' });
        flags.push(if self.user_access() { 'u' } else { 's' });
        flags.push(if self.dirty() { 'd' } else { '-' });
        flags.push(if self.cacheable() { 'c' } else { '-' });
        flags.push(if self.is_page() { 'f' } else { '-' });
        write!(f, "PageEntry({:?}, {})", self.address(), flags)
    }
}

impl PageEntry<pml::Pt> {
    pub fn get(&self) -> Option<Page4K> {
        if !self.is_present() {
            return None;
        }

        let paddr = self.address();
        assert!(paddr.is_aligned_to(Page4K::ALIGN));

        Some(Page4K(paddr))
    }

    pub unsafe fn take(&mut self) -> Option<Page4K> {
        if !self.is_present() {
            return None;
        }

        let paddr = self.address();
        assert!(paddr.is_aligned_to(Page4K::ALIGN));
        let page = Page4K(paddr);
        self.set_address(PhysicalAddress::null());
        self.set_present(false);

        Some(page)
    }

    pub unsafe fn set(&mut self, page: Page4K) -> Option<Page4K> {
        let previous = unsafe { self.take() };

        self.set_address(page.0);
        self.set_present(true);

        return previous;
    }
}

impl PageEntry<pml::Pd> {
    pub unsafe fn get(&self, phys_to_virt: fn(PhysicalAddress) -> VirtualAddress) -> Option<TableOrPage<&PageTable, Page2M>> {
        if !self.is_present() {
            return None;
        }

        let paddr = self.address();
        Some(if self.is_page() {
            assert!(paddr.is_aligned_to(Page2M::ALIGN));
            TableOrPage::Page(Page2M(paddr))
        } else {
            assert!(paddr.is_aligned_to(align_of::<PageTable>()));
            let vaddr = phys_to_virt(paddr);
            let table = unsafe { (vaddr.0 as *const PageTable).as_ref().unwrap() };
            TableOrPage::Table(table)
        })
    }

    pub unsafe fn take(&self) -> Option<TableOrPage<PageTable, Page2M>> {
        todo!();
    }

    pub unsafe fn set(&self, _table_or_page: TableOrPage<PageTable, Page2M>) -> Option<TableOrPage<PageTable, Page2M>> {
        todo!();
    }
}

impl PageEntry<pml::Pdpt> {
    pub unsafe fn get(&self, phys_to_virt: fn(PhysicalAddress) -> VirtualAddress) -> Option<TableOrPage<&PageDirectory, Page1G>> {
        if !self.is_present() {
            return None;
        }

        let paddr = self.address();
        Some(if self.is_page() {
            assert!(paddr.is_aligned_to(Page1G::ALIGN));
            TableOrPage::Page(Page1G(paddr))
        } else {
            assert!(paddr.is_aligned_to(align_of::<PageDirectory>()));
            let vaddr = phys_to_virt(paddr);
            let table = unsafe { (vaddr.0 as *const PageDirectory).as_ref().unwrap() };
            TableOrPage::Table(table)
        })
    }
}

impl PageEntry<pml::Pml4> {
    pub unsafe fn get(&self, phys_to_virt: fn(PhysicalAddress) -> VirtualAddress) -> Option<&PageDirectoryPointerTable> {
        if !self.is_present() {
            return None;
        }

        assert!(!self.is_page());

        let paddr = self.address();
        assert!(paddr.is_aligned_to(align_of::<PageDirectoryPointerTable>()));
        let vaddr = phys_to_virt(paddr);
        let table = unsafe { (vaddr.0 as *const PageDirectoryPointerTable).as_ref().unwrap() };
        Some(table)
    }
}

impl PageEntry<pml::Pml5> {
    pub unsafe fn get(&self, phys_to_virt: fn(PhysicalAddress) -> VirtualAddress) -> Option<&PageMapLevel4> {
        if !self.is_present() {
            return None;
        }

        assert!(!self.is_page());

        let paddr = self.address();
        assert!(paddr.is_aligned_to(align_of::<PageMapLevel4>()));
        let vaddr = phys_to_virt(paddr);
        let table = unsafe { (vaddr.0 as *const PageMapLevel4).as_ref().unwrap() };
        Some(table)
    }
}
