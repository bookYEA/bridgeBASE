use anchor_lang::prelude::*;

pub const INCOMING_MESSAGE_SEED: &[u8] = b"incoming_message";
pub const OUTPUT_ROOT_SEED: &[u8] = b"output_root";
pub const BRIDGE_CPI_AUTHORITY_SEED: &[u8] = b"bridge_cpi_authority";
pub const WRAPPED_TOKEN_SEED: &[u8] = b"wrapped_token";

pub const REMOTE_TOKEN_METADATA_KEY: &str = "remote_token";
pub const SCALER_EXPONENT_METADATA_KEY: &str = "scaler_exponent";

mod private {
    use super::*;

    #[cfg(feature = "devnet")]
    pub mod config {
        use super::*;

        pub const TRUSTED_ORACLE: Pubkey = pubkey!("eEwCrQLBdQchykrkYitkYUZskd7MPrU2YxBXcPDPnMt");
    }

    #[cfg(feature = "mainnet")]
    pub mod config {
        use super::*;

        pub const TRUSTED_ORACLE: Pubkey = pubkey!("11111111111111111111111111111111");
    }

    #[cfg(not(any(feature = "devnet", feature = "mainnet")))]
    pub mod config {
        use super::*;

        pub const TRUSTED_ORACLE: Pubkey = pubkey!("CB8GXDdZDSD5uqfeow1qfp48ouayxXGpw7ycmoovuQMX");
    }
}

pub use private::config::*;
