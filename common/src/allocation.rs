use core::{
    alloc::{GlobalAlloc, Layout},
    cell::UnsafeCell,
    marker::PhantomData,
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering},
};

use alloc::alloc::{AllocError, Allocator};

pub trait PageAllocator {
    fn alloc(&self, pages: usize) -> *mut u8;
    fn dealloc(&self, ptr: *mut u8, pages: usize);
}

unsafe impl<const SIZE: usize> Sync for FixedBufferAllocator<SIZE> {}

#[repr(C, align(4096))]
pub struct FixedBufferAllocator<const SIZE: usize> {
    heap: UnsafeCell<[u8; SIZE]>,
    remaining: AtomicUsize,
}

impl<const SIZE: usize> FixedBufferAllocator<SIZE> {
    const MAX_SUPPORTED_ALIGN: usize = 4096;

    pub const fn new() -> Self {
        Self {
            heap: UnsafeCell::new([0x55; SIZE]),
            remaining: AtomicUsize::new(SIZE),
        }
    }
}

unsafe impl<const SIZE: usize> Allocator for FixedBufferAllocator<SIZE> {
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

unsafe impl<const SIZE: usize> GlobalAlloc for FixedBufferAllocator<SIZE> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.allocate(layout).unwrap().as_ptr().as_mut_ptr()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe {
            self.deallocate(NonNull::new(ptr).unwrap(), layout);
        }
    }
}

unsafe impl Sync for LinearAllocator<'_> {}

pub struct LinearAllocator<'a> {
    buffer: *mut u8,
    remaining: AtomicUsize,
    _marker: PhantomData<&'a [u8]>,
}

impl<'a> LinearAllocator<'a> {
    const MAX_ALIGN: usize = 4096;

    pub fn new(buffer: &'a mut [u8]) -> Self {
        let size = buffer.len();
        let ptr = buffer.as_mut_ptr();
        assert!(size % 4096 == 0);
        assert!(ptr as usize % 4096 == 0);
        Self {
            buffer: ptr,
            remaining: AtomicUsize::new(size),
            _marker: PhantomData,
        }
    }

    pub fn remaining(&self) -> usize {
        self.remaining.load(Ordering::Relaxed)
    }
}

unsafe impl Allocator for LinearAllocator<'_> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let size = layout.size();
        let align = layout.align();

        if align > Self::MAX_ALIGN {
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

        let ptr = unsafe { self.buffer.add(allocated) };
        let ptr = NonNull::slice_from_raw_parts(NonNull::new(ptr).unwrap(), layout.size());
        Ok(ptr)
    }

    unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) {}
}

unsafe impl GlobalAlloc for LinearAllocator<'_> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.allocate(layout).unwrap().as_ptr().as_mut_ptr()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe {
            self.deallocate(NonNull::new(ptr).unwrap(), layout);
        }
    }
}
