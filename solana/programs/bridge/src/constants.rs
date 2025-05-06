use anchor_lang::prelude::*;

#[constant]
pub const DEPOSIT_VERSION: u64 = 0;

#[constant]
/// @notice Current message version identifier.
pub const MESSAGE_VERSION: u16 = 1;

#[constant]
// L2CrossDomainMessenger at 0x0580a385912cb1894b4369be2f94f2f3d6bd939a (baseSepolia)
pub const OTHER_MESSENGER: [u8; 20] = [
    5, 128, 163, 133, 145, 44, 177, 137, 75, 67, 105, 190, 47, 148, 242, 243, 214, 189, 147, 154,
];

#[constant]
// L2StandardBridge at 0xedb3c5ab354fdd99a6e1a796117f6dc15eaf316c (baseSepolia)
pub const OTHER_BRIDGE: [u8; 20] = [
    237, 179, 197, 171, 53, 79, 221, 153, 166, 225, 167, 150, 17, 127, 109, 193, 94, 175, 49, 108,
];

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
