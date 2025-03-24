#[repr(C)]
#[derive(PartialEq, Eq)]
pub struct Status(usize);

impl Status {
    pub const SUCCESS: Status = Status(0);

    #[inline(always)]
    fn highest_bit(&self) -> usize {
        self.0 >> (8 * core::mem::size_of::<usize>() - 1)
    }

    #[inline(always)]
    fn two_highest_bits(&self) -> usize {
        self.0 >> (8 * core::mem::size_of::<usize>() - 2)
    }

    #[inline]
    pub fn is_error(&self) -> bool {
        self.highest_bit() == 0b1
    }

    #[inline]
    pub fn is_efi_error(&self) -> bool {
        self.two_highest_bits() == 0b10
    }

    #[inline]
    pub fn is_oem_error(&self) -> bool {
        self.two_highest_bits() == 0b11
    }

    #[inline]
    pub fn is_warning(&self) -> bool {
        self.highest_bit() == 0b0 && *self != Self::SUCCESS
    }

    #[inline]
    pub fn is_efi_warning(&self) -> bool {
        self.two_highest_bits() == 0b00 && *self != Self::SUCCESS
    }

    #[inline]
    pub fn is_oem_warning(&self) -> bool {
        self.two_highest_bits() == 0b01 && *self != Self::SUCCESS
    }

    #[inline]
    pub fn is_success(&self) -> bool {
        self.highest_bit() == 0b0
    }
}
