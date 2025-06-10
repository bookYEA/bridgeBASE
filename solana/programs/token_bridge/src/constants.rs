use anchor_lang::prelude::*;
use hex_literal::hex;

#[constant]
pub const SOL_VAULT_SEED: &[u8] = b"sol_vault";

#[constant]
pub const BRIDGE_AUTHORITY_SEED: &[u8] = b"bridge_authority";

#[constant]
pub const NATIVE_SOL_PUBKEY: Pubkey = pubkey!("SoL1111111111111111111111111111111111111111");

#[constant]
pub const REMOTE_BRIDGE: [u8; 20] = hex!("C7ae1af5aFd9ED2E65495BFdF4639FbDB3a2ab57");

#[constant]
pub const TOKEN_VAULT_SEED: &[u8] = b"token_vault";

#[constant]
pub const WRAPPED_TOKEN_SEED: &[u8] = b"wrapped_token";

#[constant]
pub const REMOTE_TOKEN_METADATA_KEY: &str = "remote_token";
