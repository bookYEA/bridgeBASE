use anchor_lang::prelude::*;
use hex_literal::hex;

#[constant]
pub const GAS_FEE_RECEIVER: Pubkey = pubkey!("H4BF4JEUcLaNTEp4ppU5YBx8buWfQKnp32UMBH25Rp2V");

#[constant]
pub const SOL_VAULT_SEED: &[u8] = b"sol_vault";

#[constant]
pub const BRIDGE_AUTHORITY_SEED: &[u8] = b"bridge_authority";

#[constant]
pub const NATIVE_SOL_PUBKEY: Pubkey = pubkey!("SoL1111111111111111111111111111111111111111");

#[constant]
// L2StandardBridge at 0xC7ae1af5aFd9ED2E65495BFdF4639FbDB3a2ab57 (baseSepolia)
pub const REMOTE_BRIDGE: [u8; 20] = hex!("C7ae1af5aFd9ED2E65495BFdF4639FbDB3a2ab57");

#[constant]
pub const TOKEN_VAULT_SEED: &[u8] = b"token_vault";

#[constant]
pub const WRAPPED_TOKEN_SEED: &[u8] = b"wrapped_token";
