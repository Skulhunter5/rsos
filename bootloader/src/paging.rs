#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PhysicalAddress(pub u64);

impl PhysicalAddress {
    pub fn null() -> Self {
        Self(0)
    }

    pub fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl core::fmt::Debug for PhysicalAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "PhysicalAddress({:x})", self.0)
    }
}

impl Into<u64> for PhysicalAddress {
    fn into(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct VirtualAddress(pub u64);

impl core::fmt::Debug for VirtualAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "VirtualAddress({:x})", self.0)
    }
}

impl Into<u64> for VirtualAddress {
    fn into(self) -> u64 {
        self.0
    }
}

// TODO: fix address mask to include "execute disable" bit etc.
#[repr(transparent)]
pub struct PageEntry(u64);

impl PageEntry {
    pub fn address(&self) -> PhysicalAddress {
        PhysicalAddress(self.0 & !0xFFF)
    }

    pub fn present(&self) -> bool {
        self.0 & (1 << 0) != 0
    }

    pub fn writable(&self) -> bool {
        self.0 & (1 << 1) != 0
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

    pub fn accessed(&self) -> bool {
        self.0 & (1 << 5) != 0
    }

    pub fn dirty(&self) -> bool {
        self.0 & (1 << 6) != 0
    }

    pub fn page_size(&self) -> bool {
        self.0 & (1 << 7) != 0
    }

    pub fn execute_disabled(&self) -> bool {
        self.0 & (1 << 63) != 0
    }
}

impl core::fmt::Debug for PageEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        const FLAG_MAP: &[(fn(&PageEntry) -> bool, &str)] = &[
            (PageEntry::present, "present"),
            (PageEntry::user_access, "user_access"),
        ];

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
        flags.push(if self.page_size() { 'f' } else { '-' });
        write!(f, "PageEntry({:?}, {})", self.address(), flags)
    }
}

pub type PageMapLevel4 = PageMapLevel<4, PageDirectoryPointer>;
pub type PageDirectoryPointer = PageMapLevel<3, PageDirectory>;
pub type PageDirectory = PageMapLevel<2, PageTable>;
pub type PageTable = PageMapLevel<1, PageEntry>;

#[derive(Debug)]
#[repr(transparent)]
pub struct PageMapLevel<const N: usize, T> {
    entries: [PageEntry; 512],
    _marker: core::marker::PhantomData<T>,
}

impl<const N: usize, T> PageMapLevel<N, T> {
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

    pub fn get(&self, index: usize) -> &PageEntry {
        &self.entries[index]
    }

    pub fn get_mut(&mut self, index: usize) -> &mut PageEntry {
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
}

impl<const N: usize, T> core::ops::Index<usize> for PageMapLevel<N, T> {
    type Output = PageEntry;

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
    pub fn pml4_address(&self) -> PhysicalAddress {
        PhysicalAddress(self.0 & !0xFFF)
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
