use alloy_primitives::address;

// Common constants (network-agnostic)
pub const REMOTE_CALL_SEED: &[u8] = b"remote_call";
pub const PORTAL_AUTHORITY_SEED: &[u8] = b"portal_authority";
pub const OUTPUT_ROOT_SEED: &[u8] = b"output_root";
pub const PORTAL_SEED: &[u8] = b"portal";

pub const EIP1559_MINIMUM_BASE_FEE: u64 = 1;
pub const EIP1559_DEFAULT_WINDOW_DURATION_SECONDS: u64 = 1;
pub const EIP1559_DEFAULT_GAS_TARGET_PER_WINDOW: u64 = 5_000_000;
pub const EIP1559_DEFAULT_ADJUSTMENT_DENOMINATOR: u64 = 2;

pub const NATIVE_ETH_TOKEN: [u8; 20] =
    address!("EeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE").into_array();

pub const GAS_COST_SCALER_DP: u64 = 10u64.pow(6);
pub const GAS_COST_SCALER: u64 = 1_000_000;

pub const WRAPPED_TOKEN_SEED: &[u8] = b"wrapped_token";

mod private {
    use anchor_lang::prelude::*;

    #[cfg(feature = "devnet")]
    pub mod config {
        use super::*;

        pub const GAS_FEE_RECEIVER: Pubkey = pubkey!("eEwCrQLBdQchykrkYitkYUZskd7MPrU2YxBXcPDPnMt");
        pub const TRUSTED_ORACLE: Pubkey = pubkey!("eEwCrQLBdQchykrkYitkYUZskd7MPrU2YxBXcPDPnMt");
        pub const TOKEN_BRIDGE: Pubkey = pubkey!("99GM6j7R186ie7izfeHaB97LWSamFHiV8EbQVTdLuhan");

        pub const GAS_FEE_RECEIVER_KEYPAIR_BASE58: &str = "";
        pub const TRUSTED_ORACLE_KEYPAIR_BASE58: &str = "";
    }

    #[cfg(feature = "mainnet")]
    pub mod config {
        use super::*;

        pub const GAS_FEE_RECEIVER: Pubkey = pubkey!("11111111111111111111111111111111");
        pub const TRUSTED_ORACLE: Pubkey = pubkey!("11111111111111111111111111111111");
        pub const TOKEN_BRIDGE: Pubkey = pubkey!("11111111111111111111111111111111");

        pub const GAS_FEE_RECEIVER_KEYPAIR_BASE58: &str = "";

        pub const TRUSTED_ORACLE_KEYPAIR_BASE58: &str = "";
    }

    #[cfg(not(any(feature = "devnet", feature = "mainnet")))]
    pub mod config {
        use super::*;

        pub const GAS_FEE_RECEIVER: Pubkey =
            pubkey!("CB8GXDdZDSD5uqfeow1qfp48ouayxXGpw7ycmoovuQMX");

        pub const TRUSTED_ORACLE: Pubkey = pubkey!("CB8GXDdZDSD5uqfeow1qfp48ouayxXGpw7ycmoovuQMX");

        pub const TOKEN_BRIDGE: Pubkey = pubkey!("99GM6j7R186ie7izfeHaB97LWSamFHiV8EbQVTdLuhan");

        pub const GAS_FEE_RECEIVER_KEYPAIR_BASE58: &str =
        "31wWhPeEw1NQmnC2kNCwHfk4eA1G1QtxH2iqAWsEpMuCb2mvSrej2HGn3nn698dviwBGRaTZBmwXRAyVSHcSQFqs";

        pub const TRUSTED_ORACLE_KEYPAIR_BASE58: &str =
            "31wWhPeEw1NQmnC2kNCwHfk4eA1G1QtxH2iqAWsEpMuCb2mvSrej2HGn3nn698dviwBGRaTZBmwXRAyVSHcSQFqs";
    }
}

pub use private::config::*;
