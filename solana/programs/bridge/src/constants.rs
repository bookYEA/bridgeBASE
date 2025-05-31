use anchor_lang::prelude::*;
use hex_literal::hex;

#[constant]
pub const DEPOSIT_VERSION: u64 = 0;

#[constant]
/// @notice Current message version identifier.
pub const MESSAGE_VERSION: u16 = 1;

#[constant]
// L2CrossDomainMessenger at 0x2c85Bb93B4c1F07E80a242FfB3Fa9c0e8b72BB00 (baseSepolia)
pub const REMOTE_MESSENGER: [u8; 20] = hex!("2c85Bb93B4c1F07E80a242FfB3Fa9c0e8b72BB00");

#[constant]
// L2StandardBridge at 0xC7ae1af5aFd9ED2E65495BFdF4639FbDB3a2ab57 (baseSepolia)
pub const REMOTE_BRIDGE: [u8; 20] = hex!("C7ae1af5aFd9ED2E65495BFdF4639FbDB3a2ab57");

#[constant]
pub const DEFAULT_MESSENGER_CALLER: [u8; 20] = hex!("000000000000000000000000000000000000dEaD");

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
pub const AUTHORITY_VAULT_SEED: &[u8] = b"authority_vault";

#[constant]
pub const TOKEN_VAULT_SEED: &[u8] = b"token_vault";

#[constant]
pub const MESSENGER_SEED: &[u8] = b"messenger";

#[constant]
pub const OUTPUT_ROOT_SEED: &[u8] = b"output_root";

#[constant]
pub const MESSAGE_SEED: &[u8] = b"message";

#[constant]
pub const DEPOSIT_SEED: &[u8] = b"deposit";

#[constant]
pub const MINT_SEED: &[u8] = b"mint";

#[constant]
pub const BRIDGE_SEED: &[u8] = b"bridge";

#[constant]
pub const NATIVE_SOL_PUBKEY: Pubkey = pubkey!("SoL1111111111111111111111111111111111111111");

#[constant]
// pub const TRUSTED_ORACLE: Pubkey = pubkey!("eEwCrQLBdQchykrkYitkYUZskd7MPrU2YxBXcPDPnMt"); // un-comment for Devnet deployments
pub const TRUSTED_ORACLE: Pubkey = pubkey!("H4BF4JEUcLaNTEp4ppU5YBx8buWfQKnp32UMBH25Rp2V"); // for local testing

#[constant]
pub const TOKEN_PROGRAM_ID: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

#[constant]
pub const FINALIZE_BRIDGE_TOKEN_SELECTOR: [u8; 4] = hex!("2d916920");

#[constant]
pub const RELAY_MESSAGE_SELECTOR: [u8; 4] = hex!("54aa43a3");

#[constant]
pub const GAS_PER_BYTE_COST: u64 = 40;

#[constant]
pub const BASE_TRANSACTION_COST: u64 = 21000;

#[constant]
pub const SOL_TO_ETH_FACTOR: u64 = 15;

#[constant]
pub const GAS_FEE_RECEIVER: Pubkey = pubkey!("H4BF4JEUcLaNTEp4ppU5YBx8buWfQKnp32UMBH25Rp2V");

#[constant]
pub const VERSION: u8 = 2;
