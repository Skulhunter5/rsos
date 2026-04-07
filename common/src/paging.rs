use core::{marker::PhantomData, mem::MaybeUninit, ptr};

use alloc::boxed::Box;

use crate::{allocation::PageAllocator, paging::page_entry::PageEntry, PhysicalAddress, VirtualAddress};

mod page_entry;

pub struct PageMap<A: PageAllocator> {
    pml4: *mut PageMapLevel4,
    page_allocator: A,
}

impl<A: PageAllocator> PageMap<A> {
    pub fn new(page_allocator: A) -> Self {
        let pml4 = PageMapLevel4::new_in(&page_allocator);
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

        let entry = self.pml4.get_mut(pml4_index);
        if !entry.present() {
            let new_pdpt = PageDirectoryPointerTable::new_in(&self.page_allocator);
            let new_pdpt: *mut PageDirectoryPointerTable = Box::into_raw(unsafe { Box::new_zeroed().assume_init() });
            let entry = PageEntry::new_present(PhysicalAddress(new_pdpt as usize));
            self.pml4.set(pml4_index, entry);
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

pub mod pml {
    pub struct Pt;
    pub struct Pd;
    pub struct Pdpt;
    pub struct Pml4;
    pub struct Pml5;
}

pub type PageTable = PageMapLevel<pml::Pt>;
pub type PageDirectory = PageMapLevel<pml::Pd>;
pub type PageDirectoryPointerTable = PageMapLevel<pml::Pdpt>;
pub type PageMapLevel4 = PageMapLevel<pml::Pml4>;
pub type PageMapLevel5 = PageMapLevel<pml::Pml5>;

#[derive(Debug)]
#[repr(C, align(4096))]
pub struct PageMapLevel<L> {
    entries: [PageEntry; 512],
    _marker: PhantomData<L>,
}

impl<L> PageMapLevel<L> {
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
}

impl PageMapLevel<pml::Pt> {
    const LEVEL: usize = 1;

    pub fn get(&self, index: usize) -> Option<Page4K> {
        let entry = self.entries.get(index)?;
        if !entry.is_present() {
            return None;
        }

        assert!(entry.is_page());

        let paddr = entry.address();
        assert!(paddr.is_aligned_to(Page4K::ALIGN));

        Some(Page4K(paddr))
    }

    pub fn get_index(vaddr: VirtualAddress) -> usize {
        const INDEX_MASK: usize = 0b1_1111_1111;
        vaddr.0 >> (INDEX_MASK.count_ones() as usize * Self::LEVEL) & INDEX_MASK
    }
}

impl PageMapLevel<pml::Pd> {
    const LEVEL: usize = 2;

    pub unsafe fn get(&self, index: usize, phys_to_virt: fn(PhysicalAddress) -> VirtualAddress) -> Option<TableOrPage<&PageMapLevel<pml::Pt>, Page2M>> {
        let entry = self.entries.get(index)?;
        if !entry.is_present() {
            return None;
        }

        let paddr = entry.address();
        Some(if entry.is_page() {
            assert!(paddr.is_aligned_to(Page2M::ALIGN));
            TableOrPage::Page(Page2M(paddr))
        } else {
            assert!(paddr.is_aligned_to(align_of::<PageMapLevel<pml::Pt>>()));
            let vaddr = phys_to_virt(paddr);
            let table = unsafe { (vaddr.0 as *const PageMapLevel<pml::Pt>).as_ref().unwrap() };
            TableOrPage::Table(table)
        })
    }

    pub fn get_index(vaddr: VirtualAddress) -> usize {
        const INDEX_MASK: usize = 0b1_1111_1111;
        vaddr.0 >> (INDEX_MASK.count_ones() as usize * Self::LEVEL) & INDEX_MASK
    }
}

impl PageMapLevel<pml::Pdpt> {
    const LEVEL: usize = 3;

    pub unsafe fn get(&self, index: usize, phys_to_virt: fn(PhysicalAddress) -> VirtualAddress) -> Option<TableOrPage<&PageMapLevel<pml::Pd>, Page1G>> {
        let entry = self.entries.get(index)?;
        if !entry.is_present() {
            return None;
        }

        let paddr = entry.address();
        Some(if entry.is_page() {
            assert!(paddr.is_aligned_to(Page1G::ALIGN));
            TableOrPage::Page(Page1G(paddr))
        } else {
            assert!(paddr.is_aligned_to(align_of::<PageMapLevel<pml::Pd>>()));
            let vaddr = phys_to_virt(paddr);
            let table = unsafe { (vaddr.0 as *const PageMapLevel<pml::Pd>).as_ref().unwrap() };
            TableOrPage::Table(table)
        })
    }

    pub fn get_index(vaddr: VirtualAddress) -> usize {
        const INDEX_MASK: usize = 0b1_1111_1111;
        vaddr.0 >> (INDEX_MASK.count_ones() as usize * Self::LEVEL) & INDEX_MASK
    }
}

impl PageMapLevel<pml::Pml4> {
    const LEVEL: usize = 4;

    pub unsafe fn get(&self, index: usize, phys_to_virt: fn(PhysicalAddress) -> VirtualAddress) -> Option<&PageDirectoryPointerTable> {
        let entry = self.entries.get(index)?;
        if !entry.is_present() {
            return None;
        }

        assert!(!entry.is_page());

        let paddr = entry.address();
        assert!(paddr.is_aligned_to(align_of::<PageDirectoryPointerTable>()));
        let vaddr = phys_to_virt(paddr);
        let table = unsafe { (vaddr.0 as *const PageDirectoryPointerTable).as_ref().unwrap() };
        Some(table)
    }

    pub fn get_index(vaddr: VirtualAddress) -> usize {
        const INDEX_MASK: usize = 0b1_1111_1111;
        vaddr.0 >> (INDEX_MASK.count_ones() as usize * Self::LEVEL) & INDEX_MASK
    }
}

#[derive(Debug)]
#[repr(C, align(4096))]
pub struct OldPageMapLevel<const N: usize, T> {
    entries: [PageEntry; 512],
    _marker: PhantomData<T>,
}

impl<const N: usize, T> OldPageMapLevel<N, T> {
    pub fn empty() -> Self {
        Self {
            entries: [PageEntry::EMPTY; 512],
            _marker: PhantomData,
        }
    }

    pub fn new_in(page_allocator: &dyn PageAllocator) -> &mut Self {
        let ptr = page_allocator.alloc(1) as *mut MaybeUninit<Self>;
        let table = unsafe { ptr.as_mut().unwrap() };
        table.write(Self::empty());
        unsafe { table.assume_init_mut() }
    }

    pub unsafe fn from_raw(address: PhysicalAddress) -> &'static mut Self {
        let address: u64 = address.into();
        unsafe { (address as *mut Self).as_mut().unwrap() }
    }

    pub fn set(&mut self, index: usize, entry: PageEntry) -> PageEntry {
        let previous_entry = self.entries[index].clone();
        self.entries[index] = entry;
        return previous_entry;
    }
}

struct OldPageMapLevel4;

impl OldPageMapLevel4 {
    pub unsafe fn get_current() -> &'static mut Self {
        let address = read_cr3().pml4_address();
        unsafe { Self::from_raw(address) }
    }
    
    unsafe fn from_raw(_address: PhysicalAddress) -> &'static mut Self {
        todo!();
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Pml4 {
    entries: [PageEntry; 512],
}

impl Pml4 {
    pub unsafe fn get_current() -> &'static mut Pml4 {
        let address = read_cr3().pml4_address();
        unsafe { Self::from_raw(address) }
    }

    pub unsafe fn from_raw(address: PhysicalAddress) -> &'static mut Pml4 {
        let address: u64 = address.into();
        unsafe { (address as *mut Pml4).as_mut().unwrap() }
    }
}

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
