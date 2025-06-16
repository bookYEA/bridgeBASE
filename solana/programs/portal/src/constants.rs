use anchor_lang::prelude::*;
use hex_literal::hex;

// Portal constants

#[cfg(any(test, feature = "test-env"))]
pub const GAS_FEE_RECEIVER_KEYPAIR_BASE58: &str =
    "31wWhPeEw1NQmnC2kNCwHfk4eA1G1QtxH2iqAWsEpMuCb2mvSrej2HGn3nn698dviwBGRaTZBmwXRAyVSHcSQFqs";

#[cfg(any(test, feature = "test-env"))]
pub const GAS_FEE_RECEIVER: Pubkey = pubkey!("CB8GXDdZDSD5uqfeow1qfp48ouayxXGpw7ycmoovuQMX");

#[cfg(not(any(test, feature = "test-env")))]
pub const GAS_FEE_RECEIVER: Pubkey = pubkey!("eEwCrQLBdQchykrkYitkYUZskd7MPrU2YxBXcPDPnMt");

pub const GAS_PER_BYTE_COST: u64 = 40;

pub const BASE_TRANSACTION_COST: u64 = 21000;

pub const SOL_TO_ETH_FACTOR: u64 = 15;

#[cfg(any(test, feature = "test-env"))]
pub const TRUSTED_ORACLE_KEYPAIR_BASE58: &str =
    "31wWhPeEw1NQmnC2kNCwHfk4eA1G1QtxH2iqAWsEpMuCb2mvSrej2HGn3nn698dviwBGRaTZBmwXRAyVSHcSQFqs";

#[cfg(any(test, feature = "test-env"))]
pub const TRUSTED_ORACLE: Pubkey = pubkey!("CB8GXDdZDSD5uqfeow1qfp48ouayxXGpw7ycmoovuQMX");

#[cfg(not(any(test, feature = "test-env")))]
pub const TRUSTED_ORACLE: Pubkey = pubkey!("eEwCrQLBdQchykrkYitkYUZskd7MPrU2YxBXcPDPnMt");

pub const REMOTE_CALL_SEED: &[u8] = b"remote_call";

pub const PORTAL_AUTHORITY_SEED: &[u8] = b"portal_authority";

pub const OUTPUT_ROOT_SEED: &[u8] = b"output_root";

pub const EIP1559_SEED: &[u8] = b"eip1559";

pub const EIP1559_INITIAL_BASE_FEE_GWEI: u64 = 1; // 1 GWEI

pub const EIP1559_DEFAULT_WINDOW_DURATION_SECONDS: u64 = 1; // 1 second windows

pub const EIP1559_DEFAULT_GAS_TARGET_PER_WINDOW: u64 = 5_000_000; // 5M gas per window

pub const EIP1559_DEFAULT_ADJUSTMENT_DENOMINATOR: u64 = 2;

// Messenger constants

pub const MESSENGER_SEED: &[u8] = b"messenger";

pub const REMOTE_MESSENGER_ADDRESS: [u8; 20] = hex!("2c85Bb93B4c1F07E80a242FfB3Fa9c0e8b72BB00");
