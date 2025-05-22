use anchor_lang::prelude::*;
use hex_literal::hex;

#[constant]
pub const DEPOSIT_VERSION: u64 = 0;

#[constant]
/// @notice Current message version identifier.
pub const MESSAGE_VERSION: u16 = 1;

#[constant]
// L2CrossDomainMessenger at 0x870e6d2d935a27b6422eE990C00F62F2860e8F26 (baseSepolia)
pub const OTHER_MESSENGER: [u8; 20] = hex!("870e6d2d935a27b6422eE990C00F62F2860e8F26");

#[constant]
// L2StandardBridge at 0x22B66c7FBC67f57d16FC94e862902098CD7b2972 (baseSepolia)
pub const OTHER_BRIDGE: [u8; 20] = hex!("22B66c7FBC67f57d16FC94e862902098CD7b2972");

#[constant]
pub const DEFAULT_SENDER: [u8; 20] = hex!("000000000000000000000000000000000000dEaD");

#[constant]
/// @notice Constant overhead added to the base gas for a message.
pub const RELAY_CONSTANT_OVERHEAD: u64 = 200_000;

#[constant]
/// @notice Gas reserved for performing the external call in `relayMessage`.
pub const RELAY_CALL_OVERHEAD: u64 = 40_000;

#[constant]
/// @notice Gas reserved for finalizing the execution of `relayMessage` after the safe call.
pub const RELAY_RESERVED_GAS: u64 = 40_000;

#[constant]
/// @notice Gas reserved for the execution between the `hasMinGas` check and the external
///         call in `relayMessage`.
pub const RELAY_GAS_CHECK_BUFFER: u64 = 5_000;

#[constant]
/// @notice Numerator for dynamic overhead added to the base gas for a message.
pub const MIN_GAS_DYNAMIC_OVERHEAD_NUMERATOR: u64 = 64;

#[constant]
/// @notice Denominator for dynamic overhead added to the base gas for a message.
pub const MIN_GAS_DYNAMIC_OVERHEAD_DENOMINATOR: u64 = 63;

#[constant]
/// @notice Overhead added to the internal message data when the full call to relayMessage is
///         ABI encoded. This is a constant value that is specific to the V1 message encoding
///         scheme. 260 is an upper bound, actual overhead can be as low as 228 bytes for an
///         empty message.
pub const ENCODING_OVERHEAD: u64 = 260;

#[constant]
/// @notice Base gas required for any transaction in the EVM.
pub const TX_BASE_GAS: u64 = 21_000;

#[constant]
/// @notice Extra gas added to base gas for each byte of calldata in a message.
pub const MIN_GAS_CALLDATA_OVERHEAD: u64 = 16;

#[constant]
/// @notice Floor overhead per byte of non-zero calldata in a message. Calldata floor was
///         introduced in EIP-7623.
pub const FLOOR_CALLDATA_OVERHEAD: u64 = 40;

#[constant]
pub const VAULT_SEED: &[u8] = b"bridge_vault";

#[constant]
pub const MESSENGER_SEED: &[u8] = b"messenger_state";

#[constant]
pub const NATIVE_SOL_PUBKEY: Pubkey = pubkey!("LYDZWqhCarLgXtQsmWFr4DaqRE7c21xd49fpdVUUaBr");

#[constant]
pub const OUTPUT_ROOT_SEED: &[u8] = b"output_root";

#[constant]
pub const MESSAGE_SEED: &[u8] = b"message";

#[constant]
pub const DEPOSIT_SEED: &[u8] = b"deposit";

#[constant]
// pub const TRUSTED_ORACLE: Pubkey = pubkey!("eEwCrQLBdQchykrkYitkYUZskd7MPrU2YxBXcPDPnMt"); // un-comment for Devnet deployments
pub const TRUSTED_ORACLE: Pubkey = pubkey!("H4BF4JEUcLaNTEp4ppU5YBx8buWfQKnp32UMBH25Rp2V"); // for local testing

#[constant]
pub const ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey =
    pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

#[constant]
pub const TOKEN_PROGRAM_ID: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

#[constant]
pub const FINALIZE_BRIDGE_TOKEN_SELECTOR: [u8; 4] = hex!("2d916920");

#[constant]
pub const GAS_PER_BYTE_COST: u64 = 40;

#[constant]
pub const BASE_TRANSACTION_COST: u64 = 21000;

#[constant]
pub const VERSION: u8 = 1;
