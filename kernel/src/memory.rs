use core::{mem::MaybeUninit, ops::Range};

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use common::PhysicalAddress;

const PAGE_SIZE: usize = 4096;

#[derive(Debug)]
pub struct PhysicalMemoryManager {
    first_region: &'static mut RegionHeader,
}

const _: () = {
    assert!(size_of::<RegionHeader>() < 4096);
};

#[derive(Debug)]
struct RegionHeader {
    meta: &'static mut [usize],
    first_page: PhysicalAddress,
    pages: usize,
    next_region: Option<&'static mut RegionHeader>,
    next_check: usize,
    pages_free: usize,
}

impl RegionHeader {
    const VALUE_BIT_COUNT: usize = size_of::<usize>() * 8;

    fn new(meta: &'static mut [usize], first_page: PhysicalAddress, pages: usize) -> Self {
        Self {
            meta,
            first_page,
            pages,
            next_region: None,
            next_check: 0,
            pages_free: pages,
        }
    }

    fn alloc_page(&mut self) -> Option<PhysicalAddress> {
        if self.pages_free == 0 {
            if let Some(next) = &mut self.next_region {
                return next.alloc_page();
            } else {
                return None;
            }
        }

        let mut current = self.next_check;
        loop {
            if !self.check_bit(current) {
                self.set_bit(current);
                self.pages_free -= 1;
                self.next_check = (current + 1) % self.pages;

                let page = self.first_page + current * PAGE_SIZE;
                return Some(page);
            }
            current = (current + 1) % self.pages;
        }
    }

    pub fn free_page(&mut self, page: PhysicalAddress) {
        if self.contains_page(page) {
            let index = (page - self.first_page) / PAGE_SIZE;
            self.unset_bit(index);
        } else if let Some(next) = &mut self.next_region {
            next.free_page(page);
        } else {
            panic!("tried to free a page not managed by this allocator");
        }
    }

    fn check_bit(&self, index: usize) -> bool {
        let (i, j) = Self::indices(index);
        let val = self.meta[i];

        val & (1 << j) != 0
    }

    fn set_bit(&mut self, index: usize) {
        let (i, j) = Self::indices(index);
        self.meta[i] |= 1 << j;
    }

    fn unset_bit(&mut self, index: usize) {
        let (i, j) = Self::indices(index);
        self.meta[i] &= !(1 << j);
    }

    fn indices(index: usize) -> (usize, usize) {
        let i = index / Self::VALUE_BIT_COUNT;
        let j = index % Self::VALUE_BIT_COUNT;
        (i, j)
    }

    fn contains_page(&self, page: PhysicalAddress) -> bool {
        assert!(page % PAGE_SIZE == 0);
        if page < self.first_page {
            return false;
        }
        if page >= self.first_page + self.pages * PAGE_SIZE {
            return false;
        }
        return true;
    }
}

impl PhysicalMemoryManager {
    pub fn init(memory: &Vec<Range<PhysicalAddress>>) -> Result<Self, String> {
        let mut last_region = None;
        for range in memory {
            let header = range.start;
            let start = header + 4096;
            let end = range.end;
            let length = end - start;
            if header.is_null() {
                // TODO: start at page one instead
                crate::println!(
                    "- skipping 0x{:x}-0x{:x} because it requires a valid pointer to null",
                    header.0,
                    end.0
                );
                continue;
            }
            if length % PAGE_SIZE != 0 {
                // TODO: shrink to page boundaries instead
                crate::println!(
                    "- skipping 0x{:x}-0x{:x} because of invalid (non-page) alignment",
                    header.0,
                    end.0
                );
                continue;
            }
            let page_count = length / PAGE_SIZE;
            let meta_pages = PAGE_SIZE / (PAGE_SIZE * 8) + 1;
            if meta_pages >= page_count {
                crate::println!(
                    "- skipping 0x{:x}-0x{:x} because it needs too many meta-pages",
                    header.0,
                    end.0
                );
                continue;
            }
            let page_count = page_count - meta_pages;

            let meta_count = meta_pages * (PAGE_SIZE / size_of::<usize>());
            let meta = start.0 as *mut usize;
            unsafe {
                meta.write_bytes(0, meta_count);
            }
            let meta = core::ptr::slice_from_raw_parts_mut(meta, meta_count);
            let meta = unsafe { meta.as_mut().unwrap() };

            let first_page = start + meta_pages * PAGE_SIZE;

            let header = unsafe {
                (header.0 as *mut MaybeUninit<RegionHeader>)
                    .as_mut()
                    .unwrap()
            };
            header.write(RegionHeader::new(meta, first_page, page_count));
            let header = unsafe { header.assume_init_mut() };

            last_region = {
                let next = last_region.take();
                header.next_region = next;
                Some(header)
            };
        }

        match last_region {
            Some(region) => Ok(Self {
                first_region: region,
            }),
            None => Err("no valid region in the provided list".to_string()),
        }
    }

    pub fn alloc_page(&mut self) -> Option<PhysicalAddress> {
        self.first_region.alloc_page()
    }

    pub fn free_page(&mut self, page: PhysicalAddress) {
        self.first_region.free_page(page);
    }
}
