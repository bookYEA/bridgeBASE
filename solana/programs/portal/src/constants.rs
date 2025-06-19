use anchor_lang::prelude::*;

// Portal constants

#[cfg(any(test, feature = "test-env"))]
pub const GAS_FEE_RECEIVER_KEYPAIR_BASE58: &str =
    "31wWhPeEw1NQmnC2kNCwHfk4eA1G1QtxH2iqAWsEpMuCb2mvSrej2HGn3nn698dviwBGRaTZBmwXRAyVSHcSQFqs";

#[cfg(any(test, feature = "test-env"))]
pub const GAS_FEE_RECEIVER: Pubkey = pubkey!("CB8GXDdZDSD5uqfeow1qfp48ouayxXGpw7ycmoovuQMX");

#[cfg(not(any(test, feature = "test-env")))]
pub const GAS_FEE_RECEIVER: Pubkey = pubkey!("eEwCrQLBdQchykrkYitkYUZskd7MPrU2YxBXcPDPnMt");

pub const GAS_COST_SCALER_DP: u64 = 10u64.pow(6);
pub const GAS_COST_SCALER: u64 = 1_000_000;

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

pub const PORTAL_SEED: &[u8] = b"portal";

pub const EIP1559_MINIMUM_BASE_FEE: u64 = 1;

pub const EIP1559_DEFAULT_WINDOW_DURATION_SECONDS: u64 = 1; // 1 second windows

pub const EIP1559_DEFAULT_GAS_TARGET_PER_WINDOW: u64 = 5_000_000; // 5M gas per window

pub const EIP1559_DEFAULT_ADJUSTMENT_DENOMINATOR: u64 = 2;
