use core::{
    alloc::{GlobalAlloc, Layout},
    cell::UnsafeCell,
    ptr::{self, NonNull},
    sync::atomic::{AtomicUsize, Ordering},
};

use alloc::alloc::{AllocError, Allocator};

use crate::uefi2;

unsafe impl<const SIZE: usize> Sync for LinearAllocator<SIZE> {}

#[repr(C, align(4096))]
pub struct LinearAllocator<const SIZE: usize> {
    heap: UnsafeCell<[u8; SIZE]>,
    remaining: AtomicUsize,
}

impl<const SIZE: usize> LinearAllocator<SIZE> {
    const MAX_SUPPORTED_ALIGN: usize = 4096;

    pub const fn new() -> Self {
        Self {
            heap: UnsafeCell::new([0x55; SIZE]),
            remaining: AtomicUsize::new(SIZE),
        }
    }
}

unsafe impl<const SIZE: usize> Allocator for LinearAllocator<SIZE> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let size = layout.size();
        let align = layout.align();

        if align > Self::MAX_SUPPORTED_ALIGN {
            return Err(AllocError);
        }

        let align_mask_to_round_down = !(align - 1);

        let mut allocated = 0;
        if self
            .remaining
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |mut remaining| {
                if size > remaining {
                    return None;
                }
                remaining -= size;
                remaining &= align_mask_to_round_down;
                allocated = remaining;
                Some(remaining)
            })
            .is_err()
        {
            return Err(AllocError);
        }

        let ptr = unsafe { self.heap.get().cast::<u8>().add(allocated) };
        let ptr = NonNull::slice_from_raw_parts(NonNull::new(ptr).unwrap(), layout.size());
        Ok(ptr)
    }

    unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) {}
}

unsafe impl<const SIZE: usize> GlobalAlloc for LinearAllocator<SIZE> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.allocate(layout).unwrap().as_ptr().as_mut_ptr()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe {
            self.deallocate(NonNull::new(ptr).unwrap(), layout);
        }
    }
}

pub trait PageAllocator {
    fn alloc(&self, pages: usize) -> *mut u8;
    fn dealloc(&self, ptr: *mut u8, pages: usize);
}

pub struct UefiPageAllocator;

impl PageAllocator for UefiPageAllocator {
    fn alloc(&self, pages: usize) -> *mut u8 {
        let ptr = uefi2::boot::allocate_pages(
            uefi2::raw::AllocateType::AllocateAnyPages,
            uefi2::raw::MemoryType::LOADER_DATA,
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
