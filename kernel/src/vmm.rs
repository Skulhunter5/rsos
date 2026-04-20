use common::{PhysicalAddress, VirtualAddress, allocation::PageAllocator, paging::{PageMap, PageMapLevel4}};

pub struct VirtualMemoryManager {
    kernel_page_map: PageMap,
}

impl VirtualMemoryManager {
    pub fn init() -> Self {
        todo!();
    }

    fn convert_page_map<A: PageAllocator>(page_allocator: A) -> PageMap<A> {
        let page_map = PageMap::new(page_allocator);

        let phys_to_virt = |paddr: PhysicalAddress| { VirtualAddress(paddr.0) };

        let pml4 = unsafe { PageMapLevel4::get_current() };
        for entry in pml4.iter() {
            match unsafe { entry.get(phys_to_virt) } {
                Some(pdpt) => {
                    for entry in pdpt.iter() {
                        todo!();
                    }
                }
                None => (),
            }
        }

        todo!();
    }
}
