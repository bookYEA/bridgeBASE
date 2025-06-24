use anchor_lang::prelude::*;

pub const SOL_VAULT_SEED: &[u8] = b"sol_vault";
pub const BRIDGE_AUTHORITY_SEED: &[u8] = b"bridge_authority";
pub const TOKEN_VAULT_SEED: &[u8] = b"token_vault";
pub const WRAPPED_TOKEN_SEED: &[u8] = b"wrapped_token";

pub const NATIVE_SOL_PUBKEY: Pubkey = pubkey!("SoL1111111111111111111111111111111111111111");

pub const REMOTE_TOKEN_METADATA_KEY: &str = "remote_token";
pub const SCALER_EXPONENT_METADATA_KEY: &str = "scaler_exponent";

mod private {
    use hex_literal::hex;

    #[cfg(feature = "devnet")]
    pub mod config {
        use super::*;
        pub const REMOTE_BRIDGE: [u8; 20] = hex!("C7ae1af5aFd9ED2E65495BFdF4639FbDB3a2ab57");
    }

    #[cfg(feature = "mainnet")]
    pub mod config {
        use super::*;
        pub const REMOTE_BRIDGE: [u8; 20] = hex!("C7ae1af5aFd9ED2E65495BFdF4639FbDB3a2ab57");
    }

    #[cfg(not(any(feature = "devnet", feature = "mainnet")))]
    pub mod config {
        use super::*;
        pub const REMOTE_BRIDGE: [u8; 20] = hex!("C7ae1af5aFd9ED2E65495BFdF4639FbDB3a2ab57");
    }
}

pub use private::config::*;
