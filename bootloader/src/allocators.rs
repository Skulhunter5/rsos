use core::{
    mem::MaybeUninit,
    ptr,
    sync::atomic::{AtomicPtr, AtomicUsize, Ordering},
};

use alloc::vec::Vec;
use common::{allocation::PageAllocator, spin::Mutex};

use crate::uefi2::{
    self,
    raw::{AllocateType, MemoryType},
};

pub struct UefiPageAllocator;

impl PageAllocator for UefiPageAllocator {
    fn alloc(&self, pages: usize) -> *mut u8 {
        let ptr = uefi2::boot::allocate_pages(
            AllocateType::AllocateAnyPages,
            MemoryType::LOADER_DATA,
            pages,
            0,
        );
        match ptr {
            Some(ptr) => ptr as *mut u8,
            None => ptr::null_mut(),
        }
    }

    fn dealloc(&self, ptr: *mut u8, pages: usize) {
        uefi2::boot::free_pages(ptr as u64, pages);
    }
}

pub struct Area<'a> {
    start: *mut u8,
    current: AtomicUsize,
    size: usize,
    next: Option<&'a Area<'a>>,
}

impl<'a> Area<'a> {
    pub fn new(start: *mut u8, size: usize, next: Option<&'a Area<'a>>) -> Self {
        Self {
            start,
            current: AtomicUsize::new(0),
            size,
            next,
        }
    }
}

impl PageAllocator for Area<'_> {
    fn alloc(&self, pages: usize) -> *mut u8 {
        let res = self
            .current
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                if current + pages <= self.size {
                    return Some(current + pages);
                } else {
                    return None;
                }
            });
        match res {
            Ok(allocated) => return unsafe { self.start.add(allocated * 4096) },
            Err(_) => {
                if let Some(next) = self.next {
                    return next.alloc(pages);
                } else {
                    return ptr::null_mut();
                }
            }
        }
    }

    fn dealloc(&self, _ptr: *mut u8, _pages: usize) {}
}

pub struct RuntimePageAllocator<'a> {
    first_area: AtomicPtr<Area<'a>>,
}

impl RuntimePageAllocator<'_> {
    pub const fn new() -> Self {
        let first_area = AtomicPtr::new(ptr::null_mut());
        Self { first_area }
    }

    pub fn setup(&self, free_memory: &Vec<(u64, u64)>) {
        let mut next = None;
        for (start, end) in free_memory.iter().rev() {
            let start = if *start == 0 { *start + 4096 } else { *start };
            let end = *end;
            assert!(start % 4096 == 0);
            assert!(end % 4096 == 0);
            assert!(end > start);

            let ptr = start as *mut MaybeUninit<Area>;
            let area = unsafe { ptr.as_mut().unwrap() };

            let start = start + 4096;
            let size = (end - start) as usize;
            assert!(size % 4096 == 0);
            let size = size / 4096;
            area.write(Area::new(start as *mut u8, size, next));

            next = Some(unsafe { area.assume_init_ref() });
        }
        if let Some(area) = next {
            let ptr = area as *const Area as *mut Area;
            self.first_area.store(ptr, Ordering::Release);
        }
    }
}

impl PageAllocator for RuntimePageAllocator<'_> {
    fn alloc(&self, pages: usize) -> *mut u8 {
        let area = unsafe {
            self.first_area
                .load(Ordering::Acquire)
                .as_ref()
                .expect("called alloc on RuntimePageAllocator before setup")
        };
        area.alloc(pages)
    }

    fn dealloc(&self, ptr: *mut u8, pages: usize) {
        let area = unsafe {
            self.first_area
                .load(Ordering::Acquire)
                .as_ref()
                .expect("called dealloc on RuntimePageAllocator before setup")
        };
        area.dealloc(ptr, pages);
    }
}

pub enum ActivePageAllocator {
    Uefi,
    Runtime,
}

impl ActivePageAllocator {
    pub fn get_page_allocator(&self) -> &dyn PageAllocator {
        match self {
            Self::Uefi => &crate::UEFI_PAGE_ALLOCATOR,
            Self::Runtime => &crate::RUNTIME_PAGE_ALLOCATOR,
        }
    }
}

pub struct BootloaderPageAllocator {
    inner: Mutex<ActivePageAllocator>,
}

impl BootloaderPageAllocator {
    pub const fn new() -> Self {
        let inner = Mutex::new(ActivePageAllocator::Uefi);
        Self { inner }
    }

    pub fn set_inner(&self, page_allocator: ActivePageAllocator) {
        let mut inner = self.inner.lock();
        *inner = page_allocator;
    }

    pub fn set_runtime(&self) {
        self.set_inner(ActivePageAllocator::Runtime);
    }
}

impl PageAllocator for BootloaderPageAllocator {
    fn alloc(&self, pages: usize) -> *mut u8 {
        self.inner.lock().get_page_allocator().alloc(pages)
    }

    fn dealloc(&self, ptr: *mut u8, pages: usize) {
        self.inner.lock().get_page_allocator().dealloc(ptr, pages)
    }
}
