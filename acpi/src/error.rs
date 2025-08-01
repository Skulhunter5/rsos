#[derive(Debug, Clone, Copy)]
pub enum AcpiError {
    InvalidChecksum,
    RsdpError(RsdpError),
}

#[derive(Debug, Clone, Copy)]
pub enum RsdpError {
    NullPointer,
    InvalidRevision(u8),
    InvalidChecksum,
    InvalidSignature([u8; 8]),
}

impl From<RsdpError> for AcpiError {
    fn from(e: RsdpError) -> Self {
        Self::RsdpError(e)
    }
}
