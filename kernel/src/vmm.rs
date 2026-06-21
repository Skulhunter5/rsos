use crate::memory::PhysicalMemoryManager;

pub struct VirtualMemoryManager {
    pmm: PhysicalMemoryManager,
}

impl VirtualMemoryManager {
    pub fn init(pmm: PhysicalMemoryManager) -> Self {
        // TODO: create PageMap, copy entries from current page table, switch to new page tables
        Self { pmm }
    }
}
