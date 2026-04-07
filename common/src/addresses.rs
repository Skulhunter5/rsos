pub type PAddr = PhysicalAddress;
pub type VAddr = VirtualAddress;
pub type PhysAddr = PhysicalAddress;
pub type VirtAddr = VirtualAddress;

macro_rules! address_impl(
    ($name:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        #[repr(transparent)]
        pub struct $name(pub usize);

        impl $name {
            pub fn null() -> Self {
                Self(0)
            }

            pub fn is_null(&self) -> bool {
                self.0 == 0
            }

            pub fn is_aligned_to(&self, align: usize) -> bool {
                (self.0 as *mut ()).is_aligned_to(align)
            }
        }

        impl core::fmt::Debug for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}(0x{:x})", stringify!($name), self.0)
            }
        }

        impl Into<usize> for $name {
            fn into(self) -> usize {
                self.0
            }
        }

        impl From<usize> for $name {
            fn from(address: usize) -> Self {
                Self(address)
            }
        }

        #[cfg(target_pointer_width = "64")]
        impl Into<u64> for $name {
            fn into(self) -> u64 {
                self.0 as u64
            }
        }

        #[cfg(target_pointer_width = "64")]
        impl From<u64> for $name {
            fn from(value: u64) -> Self {
                Self(value as usize)
            }
        }

        impl core::ops::Add<usize> for $name {
            type Output = Self;

            fn add(self, rhs: usize) -> Self::Output {
                Self(self.0 + rhs)
            }
        }

        impl core::ops::Sub<Self> for $name {
            type Output = usize;

            fn sub(self, rhs: Self) -> Self::Output {
                self.0 - rhs.0
            }
        }

        impl core::ops::Div<usize> for $name {
            type Output = usize;

            fn div(self, rhs: usize) -> Self::Output {
                self.0 / rhs
            }
        }

        impl core::ops::Rem<usize> for $name {
            type Output = usize;

            fn rem(self, rhs: usize) -> Self::Output {
                self.0 % rhs
            }
        }
    }
);

address_impl!(PhysicalAddress);
address_impl!(VirtualAddress);
