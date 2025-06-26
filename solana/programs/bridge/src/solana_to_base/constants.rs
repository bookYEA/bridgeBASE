pub const OUTGOING_MESSAGE_SEED: &[u8] = b"outgoing_message";

pub const GAS_COST_SCALER_DP: u64 = 10u64.pow(6);
pub const GAS_COST_SCALER: u64 = 1_000_000;

mod private {
    use anchor_lang::prelude::*;

    #[cfg(feature = "devnet")]
    pub mod config {
        use super::*;

        pub const GAS_FEE_RECEIVER: Pubkey = pubkey!("eEwCrQLBdQchykrkYitkYUZskd7MPrU2YxBXcPDPnMt");
    }

    #[cfg(feature = "mainnet")]
    pub mod config {
        use super::*;

        pub const GAS_FEE_RECEIVER: Pubkey = pubkey!("11111111111111111111111111111111");
    }

    #[cfg(not(any(feature = "devnet", feature = "mainnet")))]
    pub mod config {
        use super::*;

        pub const GAS_FEE_RECEIVER: Pubkey =
            pubkey!("CB8GXDdZDSD5uqfeow1qfp48ouayxXGpw7ycmoovuQMX");
    }
}

pub use private::config::*;
