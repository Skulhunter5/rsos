use core::{marker::PhantomData, mem::MaybeUninit};

use alloc::boxed::Box;

use crate::{allocation::PageAllocator, paging::page_entry::PageEntry, PhysicalAddress, VirtualAddress};

mod page_entry;

pub struct PageMap {
    pml4: Box<PageMapLevel4>
}

impl PageMap {
    pub fn new() -> Self {
        let pml4 = Box::new_uninit();
        let pml4 = unsafe { pml4.assume_init() };
        Self { pml4 }
    }

    pub fn map(&mut self, vaddr: VirtualAddress, paddr: PhysicalAddress) -> PhysicalAddress {
        // let mut entry = PageEntry::empty();
        // entry.set_address(paddr);
        // let old_entry = self.pml4.set(PageMapLevel4::get_index(vaddr), entry);
        // return old_entry.address();

        let pml4_index = PageMapLevel4::get_index(vaddr);
        if !self.pml4.is_present(pml4_index) {
            let new_pdpt: *mut PageDirectoryPointerTable = Box::into_raw(unsafe { Box::new_zeroed().assume_init() });
            let entry = PageEntry::new_present(PhysicalAddress(new_pdpt as usize));
            self.pml4.set(pml4_index, entry);
        }
        let pdpt = self.pml4.entries[pml4_index].address();

        todo!();
    }
}

pub type PageMapLevel4 = PageMapLevel<4, PageDirectoryPointerTable>;
pub type PageDirectoryPointerTable = PageMapLevel<3, PageDirectory>;
pub type PageDirectory = PageMapLevel<2, PageTable>;
pub type PageTable = PageMapLevel<1, u8>;

#[derive(Debug)]
#[repr(C, align(4096))]
pub struct PageMapLevel<const N: usize, T> {
    entries: [PageEntry<N>; 512],
    _marker: PhantomData<T>,
}

const _: () = {
    assert!(size_of::<PageMapLevel4>() == 4096);
    assert!(core::mem::offset_of!(PageMapLevel4, entries) == 0);
};

impl<const N: usize, T> PageMapLevel<N, T> {
    pub fn empty() -> Self {
        Self {
            entries: [PageEntry::empty(); 512],
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

    pub fn count_present(&self) -> usize {
        let mut count = 0;
        for entry in &self.entries {
            if entry.present() {
                count += 1;
            }
        }

        count
    }

    pub fn is_present(&self, index: usize) -> bool {
        self.entries[index].present()
    }

    pub fn get(&self, index: usize) -> &PageEntry<N> {
        &self.entries[index]
    }

    pub fn set(&mut self, index: usize, entry: PageEntry<N>) -> PageEntry<N> {
        let previous_entry = self.entries[index].clone();
        self.entries[index] = entry;
        return previous_entry;
    }

    pub fn get_mut(&mut self, index: usize) -> &mut PageEntry<N> {
        &mut self.entries[index]
    }

    pub fn next_level(
        &mut self,
        index: usize,
        phys_to_virt: fn(PhysicalAddress) -> VirtualAddress,
    ) -> &mut T {
        let paddr = self.get(index).address();
        if paddr.is_null() {
            panic!("tried getting next page map level from absent entry");
        }
        let vaddr = phys_to_virt(paddr);
        let ptr = vaddr.0 as *mut T;
        unsafe { ptr.as_mut().unwrap() }
    }

    pub fn get_index(vaddr: VirtualAddress) -> usize {
        const INDEX_MASK: usize = 0b1_1111_1111;
        vaddr.0 >> (INDEX_MASK.count_ones() as usize * N) & INDEX_MASK
    }
}

impl<const N: usize, T> core::ops::Index<usize> for PageMapLevel<N, T> {
    type Output = PageEntry<N>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl<const N: usize, T> core::ops::IndexMut<usize> for PageMapLevel<N, T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

impl PageMapLevel4 {
    pub unsafe fn get_current() -> &'static mut Self {
        let address = read_cr3().pml4_address();
        unsafe { Self::from_raw(address) }
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Pml4 {
    entries: [PageEntry<4>; 512],
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
