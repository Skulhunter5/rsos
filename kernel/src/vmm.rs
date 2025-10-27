use common::paging::PageMap;

pub struct VirtualMemoryManager {
    kernel_page_map: PageMap,
}

impl VirtualMemoryManager {
    pub fn init() -> Self {
        todo!();
    }
}
