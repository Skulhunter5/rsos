use core::mem;

#[repr(C)]
#[must_use = "this `Status` may be an error, which should be handled"]
#[derive(PartialEq, Eq)]
pub struct Status(usize);

impl Status {
    // Success
    pub const SUCCESS: Self = Self(0);
    // EFI Errors
    pub const LOAD_ERROR: Self = Self::new_efi_error(1);
    pub const INVALID_PARAMETER: Self = Self::new_efi_error(2);
    pub const UNSUPPORTED: Self = Self::new_efi_error(3);
    pub const BAD_BUFFER_SIZE: Self = Self::new_efi_error(4);
    pub const BUFFER_TOO_SMALL: Self = Self::new_efi_error(5);
    // TODO: add remaining specified statuses from UEFI spec

    pub const HIGHEST_BIT: usize = 1 << (8 * mem::size_of::<usize>() - 1);

    #[inline(always)]
    const fn new_efi_error(code: usize) -> Self {
        Self(Self::HIGHEST_BIT | code)
    }

    #[inline(always)]
    fn highest_bit(&self) -> usize {
        self.0 >> (8 * mem::size_of::<usize>() - 1)
    }

    #[inline(always)]
    fn two_highest_bits(&self) -> usize {
        self.0 >> (8 * mem::size_of::<usize>() - 2)
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
