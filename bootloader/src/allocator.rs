use core::{alloc::GlobalAlloc, cell::UnsafeCell, sync::atomic::{AtomicUsize, Ordering}};

unsafe impl Sync for LinearAllocator {}

#[repr(C, align(4096))]
pub struct LinearAllocator {
    heap: UnsafeCell<[u8; Self::HEAP_SIZE]>,
    remaining: AtomicUsize,
}

impl LinearAllocator {
    const MAX_SUPPORTED_ALIGN: usize = 4096;
    const HEAP_SIZE: usize = 16 * 1024 * 1024;

    pub const fn new() -> Self {
        Self { heap: UnsafeCell::new([0x55; Self::HEAP_SIZE]), remaining: AtomicUsize::new(Self::HEAP_SIZE) }
    }
}

unsafe impl GlobalAlloc for LinearAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();

        if align > Self::MAX_SUPPORTED_ALIGN {
            return core::ptr::null_mut();
        }

        let align_mask_to_round_down = !(align - 1);

        let mut allocated = 0;
        if self.remaining.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |mut remaining| {
            if size > remaining {
                return None;
            }
            remaining -= size;
            remaining &= align_mask_to_round_down;
            allocated = remaining;
            Some(remaining)
        }).is_err() {
            return core::ptr::null_mut();
        }

        unsafe { self.heap.get().cast::<u8>().add(allocated) }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {}
}
