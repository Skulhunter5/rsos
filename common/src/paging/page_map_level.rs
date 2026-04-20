use core::{marker::PhantomData, mem::MaybeUninit, ptr};

use crate::{PhysicalAddress, VirtualAddress, allocation::PageAllocator};

use super::{page_entry::PageEntry, Page4K, Page2M, Page1G, TableOrPage};

pub mod pml {
    pub struct Pt;
    pub struct Pd;
    pub struct Pdpt;
    pub struct Pml4;
    pub struct Pml5;
}

pub(super) trait IPageMapLevel {
    const LEVEL: usize;
}

impl IPageMapLevel for pml::Pt {
    const LEVEL: usize = 1;
}

impl IPageMapLevel for pml::Pd {
    const LEVEL: usize = 2;
}

impl IPageMapLevel for pml::Pdpt {
    const LEVEL: usize = 3;
}

impl IPageMapLevel for pml::Pml4 {
    const LEVEL: usize = 4;
}

impl IPageMapLevel for pml::Pml5 {
    const LEVEL: usize = 5;
}

pub type PageTable = PageMapLevel<pml::Pt>;
pub type PageDirectory = PageMapLevel<pml::Pd>;
pub type PageDirectoryPointerTable = PageMapLevel<pml::Pdpt>;
pub type PageMapLevel4 = PageMapLevel<pml::Pml4>;
pub type PageMapLevel5 = PageMapLevel<pml::Pml5>;

#[derive(Debug)]
#[repr(C, align(4096))]
pub struct PageMapLevel<L: IPageMapLevel> {
    entries: [PageEntry<L>; 512],
    _marker: PhantomData<L>,
}

impl<L: IPageMapLevel> PageMapLevel<L> {
    pub const EMPTY: Self = Self { entries: [PageEntry::EMPTY; 512], _marker: PhantomData };

    const ASSERTIONS: () = {
        assert!(size_of::<PageMapLevel<L>>() == 4096);
        assert!(align_of::<PageMapLevel<L>>() == 4096);
        assert!(core::mem::offset_of!(PageMapLevel<L>, entries) == 0);
    };

    pub unsafe fn new_in(page_allocator: &dyn PageAllocator) -> *mut Self {
        // const _: () = {
        //     assert!(size_of::<PageMapLevel<()>>() == 4096);
        //     assert!(align_of::<PageMapLevel<()>>() == 4096);
        //     assert!(core::mem::offset_of!(PageMapLevel<()>, entries) == 0);
        // };

        let ptr = page_allocator.alloc(1) as *mut MaybeUninit<Self>;
        // SAFETY: MaybeUninit<Self> has the same layout as Self, which is asserted to have size
        // and align matching the allocated 4K page
        let table = unsafe { ptr.as_mut().unwrap() };
        table.write(Self::EMPTY);
        // SAFETY: `table` has just been initialized to be a valid value
        ptr::from_mut(unsafe { table.assume_init_mut() })
    }

    pub fn count_present(&self) -> usize {
        self.entries.iter().filter(|entry| entry.is_present()).count()
    }

    pub fn is_entry_present(&self, index: usize) -> bool {
        self.entries.get(index).map(|entry| entry.is_present()).unwrap_or(false)
    }

    pub fn get_index(vaddr: VirtualAddress) -> usize {
        const INDEX_MASK_BIT_COUNT: usize = 9;
        const INDEX_MASK: usize = (1 << INDEX_MASK_BIT_COUNT) - 1;
        vaddr.0 >> (INDEX_MASK_BIT_COUNT * L::LEVEL) & INDEX_MASK
    }

    pub fn get(&self, index: usize) -> Option<&PageEntry<L>> {
        self.entries.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut PageEntry<L>> {
        self.entries.get_mut(index)
    }

    pub fn iter(&self) -> core::slice::Iter<PageEntry<L>> {
        self.entries.iter()
    }
}

impl PageMapLevel4 {
    pub unsafe fn get_current() -> &'static Self {
        let address = super::read_cr3().pml4_address();
        let address: u64 = address.into();
        unsafe { (address as *mut Self).as_mut().unwrap() }
    }
}

// impl PageMapLevel<pml::Pt> {
//     pub fn get(&self, index: usize) -> Option<Page4K> {
//         let entry = self.entries.get(index)?;
//
//         if !entry.is_present() {
//             return None;
//         }
//
//         assert!(entry.is_page());
//
//         let paddr = entry.address();
//         assert!(paddr.is_aligned_to(Page4K::ALIGN));
//
//         Some(Page4K(paddr))
//     }
// }
//
// impl PageMapLevel<pml::Pd> {
//     pub unsafe fn get(&self, index: usize, phys_to_virt: fn(PhysicalAddress) -> VirtualAddress) -> Option<TableOrPage<&PageMapLevel<pml::Pt>, Page2M>> {
//         let entry = self.entries.get(index)?;
//         if !entry.is_present() {
//             return None;
//         }
//
//         let paddr = entry.address();
//         Some(if entry.is_page() {
//             assert!(paddr.is_aligned_to(Page2M::ALIGN));
//             TableOrPage::Page(Page2M(paddr))
//         } else {
//             assert!(paddr.is_aligned_to(align_of::<PageMapLevel<pml::Pt>>()));
//             let vaddr = phys_to_virt(paddr);
//             let table = unsafe { (vaddr.0 as *const PageMapLevel<pml::Pt>).as_ref().unwrap() };
//             TableOrPage::Table(table)
//         })
//     }
// }
//
// impl PageMapLevel<pml::Pdpt> {
//     pub unsafe fn get(&self, index: usize, phys_to_virt: fn(PhysicalAddress) -> VirtualAddress) -> Option<TableOrPage<&PageMapLevel<pml::Pd>, Page1G>> {
//         let entry = self.entries.get(index)?;
//         if !entry.is_present() {
//             return None;
//         }
//
//         let paddr = entry.address();
//         Some(if entry.is_page() {
//             assert!(paddr.is_aligned_to(Page1G::ALIGN));
//             TableOrPage::Page(Page1G(paddr))
//         } else {
//             assert!(paddr.is_aligned_to(align_of::<PageMapLevel<pml::Pd>>()));
//             let vaddr = phys_to_virt(paddr);
//             let table = unsafe { (vaddr.0 as *const PageMapLevel<pml::Pd>).as_ref().unwrap() };
//             TableOrPage::Table(table)
//         })
//     }
// }
//
// impl PageMapLevel<pml::Pml4> {
//     pub unsafe fn get(&self, index: usize, phys_to_virt: fn(PhysicalAddress) -> VirtualAddress) -> Option<&PageDirectoryPointerTable> {
//         let entry = self.entries.get(index)?;
//         if !entry.is_present() {
//             return None;
//         }
//
//         assert!(!entry.is_page());
//
//         let paddr = entry.address();
//         assert!(paddr.is_aligned_to(align_of::<PageDirectoryPointerTable>()));
//         let vaddr = phys_to_virt(paddr);
//         let table = unsafe { (vaddr.0 as *const PageDirectoryPointerTable).as_ref().unwrap() };
//         Some(table)
//     }
// }
//
// impl PageMapLevel<pml::Pml5> {
//     pub unsafe fn get(&self, index: usize, phys_to_virt: fn(PhysicalAddress) -> VirtualAddress) -> Option<&PageMapLevel4> {
//         let entry = self.entries.get(index)?;
//         if !entry.is_present() {
//             return None;
//         }
//
//         assert!(!entry.is_page());
//
//         let paddr = entry.address();
//         assert!(paddr.is_aligned_to(align_of::<PageMapLevel4>()));
//         let vaddr = phys_to_virt(paddr);
//         let table = unsafe { (vaddr.0 as *const PageMapLevel4).as_ref().unwrap() };
//         Some(table)
//     }
// }
