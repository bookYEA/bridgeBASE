use anchor_lang::prelude::*;
use hex_literal::hex;

// Portal constants

#[constant]
pub const GAS_FEE_RECEIVER: Pubkey = pubkey!("H4BF4JEUcLaNTEp4ppU5YBx8buWfQKnp32UMBH25Rp2V");

#[constant]
pub const GAS_PER_BYTE_COST: u64 = 40;

#[constant]
pub const BASE_TRANSACTION_COST: u64 = 21000;

#[constant]
pub const SOL_TO_ETH_FACTOR: u64 = 15;

#[constant]
pub const OUTPUT_ROOT_SEED: &[u8] = b"output_root";

#[constant]
// pub const TRUSTED_ORACLE: Pubkey = pubkey!("eEwCrQLBdQchykrkYitkYUZskd7MPrU2YxBXcPDPnMt"); // un-comment for Devnet deployments
pub const TRUSTED_ORACLE: Pubkey = pubkey!("H4BF4JEUcLaNTEp4ppU5YBx8buWfQKnp32UMBH25Rp2V"); // for local testing

#[constant]
pub const REMOTE_CALL_SEED: &[u8] = b"remote_call";

#[constant]
pub const PORTAL_AUTHORITY_SEED: &[u8] = b"portal_authority";

// Messenger constants

#[constant]
pub const MESSENGER_SEED: &[u8] = b"messenger";

#[constant]
pub const REMOTE_MESSENGER_ADDRESS: [u8; 20] = hex!("2c85Bb93B4c1F07E80a242FfB3Fa9c0e8b72BB00");
