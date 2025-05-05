use core::mem;

use super::SdtHeader;

#[derive(Debug)]
#[repr(C)]
pub struct Rsdt {
    pub header: SdtHeader,
    pub tables: [u32],
}

impl Rsdt {
    pub unsafe fn validate(&self) -> bool {
        let ptr = self as *const Self;
        let ptr = ptr as *const u8;
        let size = size_of_val(self);

        let sum = (0..size)
            .map(|i| unsafe { *ptr.add(i) } as usize)
            .sum::<usize>();
        return sum & 0xFF == 0;
    }
}

// TODO: implement the Debug by hand
// #[derive(Debug)]
#[repr(C, packed)]
pub struct Xsdt {
    pub header: SdtHeader,
    pub tables: [u64],
}

impl Xsdt {
    pub unsafe fn validate(&self) -> bool {
        let ptr = self as *const Self;
        let ptr = ptr as *const u8;
        let size = size_of_val(self);

        let sum = (0..size)
            .map(|i| unsafe { *ptr.add(i) } as usize)
            .sum::<usize>();
        return sum & 0xFF == 0;
    }

    pub fn len(&self) -> usize {
        (self.header.length as usize - mem::size_of::<SdtHeader>()) / mem::size_of::<u64>()
    }

    // TODO: decide whether this function should be/has to be unsafe or safe
    pub fn get(&self, index: usize) -> Option<*const SdtHeader> {
        if index >= self.len() {
            return None;
        }
        let ptr = unsafe { (self as *const Xsdt as *const u64).byte_add(mem::size_of::<SdtHeader>()) };
        let item = unsafe { ptr.read_unaligned() } as *const SdtHeader;
        return Some(item);
    }
}
