mod rsdp;
mod rsdt;
mod sdt;

pub use rsdp::{Rsdp, Xsdp};
pub use rsdt::{Rsdt, Xsdt};
pub use sdt::{Mcfg, ConfigurationSpaceBaseAddressAllocation, SdtHeader};

pub const RSDP_SIGNATURE: &[u8] = "RSD PTR ".as_bytes();
pub const MCFG_SIGNATURE: [u8; 4] = [b'M', b'C', b'F', b'G'];
