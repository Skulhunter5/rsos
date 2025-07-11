#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]

macro_rules! ops_impl (
    ($($n:ty),* $(,)?) => {
        $(
            impl core::ops::Add<ByteSize<$n>> for ByteSize<$n> {
                type Output = ByteSize<$n>;

                fn add(self, rhs: ByteSize<$n>) -> Self::Output {
                    ByteSize(self.0 + rhs.0)
                }
            }

            impl core::ops::Mul<$n> for ByteSize<$n> {
                type Output = ByteSize<$n>;

                fn mul(self, rhs: $n) -> Self::Output {
                    ByteSize(self.0 * rhs)
                }
            }

            impl core::ops::Mul<ByteSize<$n>> for $n {
                type Output = ByteSize<$n>;

                fn mul(self, rhs: ByteSize<$n>) -> Self::Output {
                    ByteSize(self * rhs.0)
                }
            }
        )*
    }
);

ops_impl!(u8, u16, u32, u64, usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ByteSize<N>(N);

impl<N> ByteSize<N>
where
    N: Clone + Copy,
{
    pub const fn from_bytes(n: N) -> Self {
        Self(n)
    }

    pub const fn raw_bytes(&self) -> N {
        self.0
    }
}

// impl core::ops::Mul<ByteSize<usize>> for usize {
//     type Output = ByteSize<usize>;
//
//     fn mul(self, rhs: ByteSize<usize>) -> Self::Output {
//         ByteSize(self * rhs.0)
//     }
// }

// impl core::ops::Add<ByteSize> for ByteSize {
//     type Output = Self;
//
//     fn add(self, rhs: ByteSize) -> Self::Output {
//         Self(self.0 + rhs.0)
//     }
// }
//
// impl core::ops::Mul<ByteSize> for ByteSize {
//     type Output = Self;
//
//     fn mul(self, rhs: ByteSize) -> Self::Output {
//         Self(self.0 * rhs.0)
//     }
// }
//
// impl core::ops::Mul<usize> for ByteSize {
//     type Output = Self;
//
//     fn mul(self, rhs: usize) -> Self::Output {
//         Self(self.0 * rhs)
//     }
// }
//
// impl Into<usize> for ByteSize {
//     fn into(self) -> usize {
//         self.0
//     }
// }

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct BitSize(usize);
