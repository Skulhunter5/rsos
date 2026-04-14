use alloc::boxed::Box;

use crate::{allocation::PageAllocator, PhysicalAddress, VirtualAddress};

mod page_entry;
mod page_map_level;
pub(self) use page_entry::{PageEntry, PageOptions};

pub(self) use page_map_level::{PageTable, PageDirectory, PageDirectoryPointerTable, PageMapLevel4, PageMapLevel5, pml};
pub(self) use page_map_level::IPageMapLevel;

pub struct PageMap<A: PageAllocator> {
    pml4: *mut PageMapLevel4,
    page_allocator: A,
}

impl<A: PageAllocator> PageMap<A> {
    pub fn new(page_allocator: A) -> Self {
        let pml4 = unsafe { PageMapLevel4::new_in(&page_allocator) };
        // let pml4 = Box::new_uninit();
        // let pml4 = unsafe { pml4.assume_init() };
        Self { pml4, page_allocator }
    }

    pub fn map(&mut self, vaddr: VirtualAddress, paddr: PhysicalAddress) -> PhysicalAddress {
        // let mut entry = PageEntry::empty();
        // entry.set_address(paddr);
        // let old_entry = self.pml4.set(PageMapLevel4::get_index(vaddr), entry);
        // return old_entry.address();

        let pml4_index = PageMapLevel4::get_index(vaddr);
        // if !self.pml4.is_present(pml4_index) {
        //     let new_pdpt: *mut PageDirectoryPointerTable = Box::into_raw(unsafe { Box::new_zeroed().assume_init() });
        //     let entry = PageEntry::new_present(PhysicalAddress(new_pdpt as usize));
        //     self.pml4.set(pml4_index, entry);
        // }
        // let pdpt = self.pml4.entries[pml4_index].address();

        let pml4 = unsafe { self.pml4.as_ref().unwrap() };

        let entry = pml4.get_mut(pml4_index).unwrap();
        if !entry.is_present() {
            let new_pdpt = unsafe { PageDirectoryPointerTable::new_in(&self.page_allocator) };
            let new_pdpt: *mut PageDirectoryPointerTable = Box::into_raw(unsafe { Box::new_zeroed().assume_init() });
            // let entry = unsafe { PageEntry::new_present(PhysicalAddress(new_pdpt as usize), PageOptions::default()) };
            entry.set_options(PageOptions::default());
            unsafe { entry.set(new_pdpt) };
            // self.pml4.set(pml4_index, entry);
        }


        // let pml4 = unsafe { PageMapLevel4::get_current() };
        // let pml4_index = (vaddr >> 39) & 0x1FF;
        //
        // let pdp = if pml4.is_present(pml4_index) {
        //     pml4.next_level(pml4_index, phys_to_virt)
        // } else {
        //     let pdp = PageDirectoryPointer::new_in(&PAGE_ALLOCATOR);
        //     let pdp_address = PhysicalAddress::from(ptr::from_ref(pdp) as usize);
        //     let mut pml4_entry = entry_template.clone();
        //     pml4_entry.set_address(pdp_address);
        //     pml4.set(pml4_index, pml4_entry);
        //
        //     pdp
        // };
        // let pdp_index = (vaddr >> 30) & 0x1FF;
        //
        // let pd = if pdp.is_present(pdp_index) {
        //     pdp.next_level(pdp_index, phys_to_virt)
        // } else {
        //     let pd = PageDirectory::new_in(&PAGE_ALLOCATOR);
        //     let pd_address = PhysicalAddress::from(ptr::from_ref(pd) as usize);
        //     let mut pdp_entry = entry_template.clone();
        //     pdp_entry.set_address(pd_address);
        //     pdp.set(pdp_index, pdp_entry);
        //
        //     pd
        // };
        // let pd_index = (vaddr >> 21) & 0x1FF;
        //
        // let pt = if pd.is_present(pd_index) {
        //     pd.next_level(pd_index, phys_to_virt)
        // } else {
        //     let pt = PageTable::new_in(&PAGE_ALLOCATOR);
        //     let pt_address = PhysicalAddress::from(ptr::from_ref(pt) as usize);
        //     let mut pd_entry = entry_template.clone();
        //     pd_entry.set_address(pt_address);
        //     pd.set(pd_index, pd_entry);
        //
        //     pt
        // };
        // let pt_index = (vaddr >> 12) & 0x1FF;
        //
        // if pt.is_present(pt_index) {
        //     panic!(
        //         "Trying to map an already present page while mapping higher-half kernel"
        //     );
        // } else {
        //     let mut pt_entry = entry_template.clone();
        //     pt_entry.set_address(PhysicalAddress::from(paddr));
        //     pt.set(pt_index, pt_entry);
        // }

        todo!();
    }
}

impl<A: PageAllocator> Drop for PageMap<A> {
    fn drop(&mut self) {
        drop(self.pml4);
        self.page_allocator.dealloc(self.pml4.cast::<u8>(), 1);
        // TODO: ensure that everything (including actual pages referenced by the tables) is freed
        // correctly
        todo!();
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
