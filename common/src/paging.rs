use crate::{allocation::PageAllocator, PhysicalAddress, VirtualAddress};

mod page_entry;
mod page_map_level;

pub use page_entry::{PageEntry, PageOptions};
pub use page_map_level::{PageTable, PageDirectory, PageDirectoryPointerTable, PageMapLevel4, PageMapLevel5, pml};

pub(self) use page_map_level::{IPageMapLevel, PageMapLevel};

pub struct PageMap<A: PageAllocator> {
    pml4: *mut PageMapLevel4,
    page_allocator: A,
}

impl<A: PageAllocator> PageMap<A> {
    pub fn new(page_allocator: A) -> Self {
        let pml4 = unsafe { PageMapLevel4::new_in(&page_allocator) };
        Self { pml4, page_allocator }
    }

    pub unsafe fn map(
        &mut self,
        vaddr: VirtualAddress,
        paddr: PhysicalAddress,
        options: PageOptions,
    ) -> PhysicalAddress {
        let pdpt = unsafe {
            self.get_or_create_child::<pml::Pml4, pml::Pdpt>(
                self.pml4,
                PageMapLevel4::get_index(vaddr),
                options,
            )
        };
        let pd = unsafe {
            self.get_or_create_child::<pml::Pdpt, pml::Pd>(
                pdpt,
                PageDirectoryPointerTable::get_index(vaddr),
                options,
            )
        };
        let pt = unsafe {
            self.get_or_create_child::<pml::Pd, pml::Pt>(
                pd,
                PageDirectory::get_index(vaddr),
                options,
            )
        };

        let pt_index = PageTable::get_index(vaddr);
        let entry = unsafe { &mut *pt }.get_mut(pt_index).unwrap();
        let old_paddr = entry.address();
        *entry = unsafe { PageEntry::new_present(paddr, options) };
        old_paddr
    }

    pub unsafe fn unmap(&mut self, vaddr: VirtualAddress) -> Option<PhysicalAddress> {
        let pdpt = unsafe {
            self.get_child::<pml::Pml4, pml::Pdpt>(
                self.pml4,
                PageMapLevel4::get_index(vaddr),
            )?
        };
        let pd = unsafe {
            self.get_child::<pml::Pdpt, pml::Pd>(
                pdpt,
                PageDirectoryPointerTable::get_index(vaddr),
            )?
        };
        let pt = unsafe {
            self.get_child::<pml::Pd, pml::Pt>(
                pd,
                PageDirectory::get_index(vaddr),
            )?
        };

        let pt_index = PageTable::get_index(vaddr);
        let entry = unsafe { &mut *pt }.get_mut(pt_index).unwrap();
        if !entry.is_present() {
            return None;
        }
        let old_paddr = entry.address();
        *entry = PageEntry::EMPTY;
        Some(old_paddr)
    }

    /// Walks one level of the page table hierarchy. If the entry at `index` in `parent`
    /// is not present, allocates a new child table, stores its physical address in the
    /// entry, and returns a pointer to the child.
    ///
    /// # Safety
    /// `parent` must point to a valid page table, `index` < 512.
    /// Assumes identity mapping (virtual == physical) for allocated pages.
    unsafe fn get_or_create_child<P: IPageMapLevel, C: IPageMapLevel>(
        &mut self,
        parent: *mut PageMapLevel<P>,
        index: usize,
        options: PageOptions,
    ) -> *mut PageMapLevel<C> {
        let entry = unsafe { &mut *parent }.get_mut(index).unwrap();
        if entry.is_present() {
            assert!(!entry.is_page(), "cannot map over an existing huge page");
            entry.address().0 as *mut PageMapLevel<C>
        } else {
            let child = unsafe { PageMapLevel::<C>::new_in(&self.page_allocator) };
            *entry = unsafe { PageEntry::new_present(PhysicalAddress::from(child as usize), options) };
            child
        }
    }

    /// Walks one level of the page table hierarchy without creating.
    /// Returns `None` if the entry is not present.
    ///
    /// # Safety
    /// `parent` must point to a valid page table, `index` < 512.
    /// Assumes identity mapping (virtual == physical) for allocated pages.
    unsafe fn get_child<P: IPageMapLevel, C: IPageMapLevel>(
        &self,
        parent: *mut PageMapLevel<P>,
        index: usize,
    ) -> Option<*mut PageMapLevel<C>> {
        let entry = unsafe { &*parent }.get(index)?;
        if !entry.is_present() {
            return None;
        }
        assert!(!entry.is_page(), "cannot traverse through an existing huge page");
        let child_paddr = entry.address();
        Some(child_paddr.0 as *mut PageMapLevel<C>)
    }

    unsafe fn free_pml4(&mut self, table: *mut PageMapLevel4) {
        for entry in unsafe { &*table }.iter() {
            if entry.is_present() {
                let child = entry.address().0 as *mut PageDirectoryPointerTable;
                unsafe { self.free_pdpt(child) };
            }
        }
        self.page_allocator.dealloc(table.cast::<u8>(), 1);
    }

    unsafe fn free_pdpt(&mut self, table: *mut PageDirectoryPointerTable) {
        for entry in unsafe { &*table }.iter() {
            if entry.is_present() && !entry.is_page() {
                let child = entry.address().0 as *mut PageDirectory;
                unsafe { self.free_pd(child) };
            }
        }
        self.page_allocator.dealloc(table.cast::<u8>(), 1);
    }

    unsafe fn free_pd(&mut self, table: *mut PageDirectory) {
        for entry in unsafe { &*table }.iter() {
            if entry.is_present() && !entry.is_page() {
                let child = entry.address().0 as *mut PageTable;
                unsafe { self.free_pt(child) };
            }
        }
        self.page_allocator.dealloc(table.cast::<u8>(), 1);
    }

    unsafe fn free_pt(&mut self, table: *mut PageTable) {
        self.page_allocator.dealloc(table.cast::<u8>(), 1);
    }
}

impl<A: PageAllocator> Drop for PageMap<A> {
    fn drop(&mut self) {
        unsafe { self.free_pml4(self.pml4) };
    }
}

#[derive(Debug)]
pub struct Page4K(pub PhysicalAddress);

impl Page4K {
    pub const SIZE: usize = 4096;
    pub const ALIGN: usize = Self::SIZE;
}

#[derive(Debug)]
pub struct Page2M(pub PhysicalAddress);

impl Page2M {
    pub const SIZE: usize = 4096 * 512;
    pub const ALIGN: usize = Self::SIZE;
}

#[derive(Debug)]
pub struct Page1G(pub PhysicalAddress);

impl Page1G {
    pub const SIZE: usize = 4096 * 512 * 512;
    pub const ALIGN: usize = Self::SIZE;
}

pub enum TableOrPage<T, P> {
    Table(T),
    Page(P),
}

// impl Pml4 {
//     pub unsafe fn get_current() -> &'static mut Pml4 {
//         let address = read_cr3().pml4_address();
//         unsafe { Self::from_raw(address) }
//     }
//
//     pub unsafe fn from_raw(address: PhysicalAddress) -> &'static mut Pml4 {
//         let address: u64 = address.into();
//         unsafe { (address as *mut Pml4).as_mut().unwrap() }
//     }
// }

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Cr3Value(u64);

impl Cr3Value {
    pub fn new(pml4_address: PhysicalAddress) -> Self {
        Self(pml4_address.0 as u64 & !0xFFF)
    }

    pub fn pml4_address(&self) -> PhysicalAddress {
        PhysicalAddress::from(self.0 & !0xFFF)
    }

    pub fn set_pml4_address(&mut self, pml4_address: PhysicalAddress) {
        self.0 = (self.0 & 0xFFF) | (pml4_address.0 as u64 & !0xFFF);
    }
}

impl core::fmt::Debug for Cr3Value {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Cr3Value {{ other: ??, pml4_address: {:?} }}",
            self.pml4_address()
        )
    }
}

pub fn read_cr3() -> Cr3Value {
    let mut val;
    unsafe {
        core::arch::asm!("mov {}, cr3", out(reg) val, options(nostack, preserves_flags));
    }
    Cr3Value(val)
}

pub fn write_cr3(val: Cr3Value) {
    let val = val.0;
    unsafe {
        core::arch::asm!("mov cr3, {}", in(reg) val, options(nostack, preserves_flags));
    }
}
