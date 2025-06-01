use anchor_lang::prelude::*;
use hex_literal::hex;

/// @notice Version identifier for deposit transactions.
///
/// @dev Used to track deposit format versions for potential future upgrades. Current version 0 represents the initial
///      deposit transaction format.
#[constant]
pub const DEPOSIT_VERSION: u64 = 0;

/// @notice Current message version identifier for cross-chain messaging.
///
/// @dev This version identifier is used to ensure compatibility between different versions of the cross-chain
///      messaging protocol. Version 1 represents the current message format and encoding scheme used for L1 <-> L2
///      communication.
#[constant]
pub const MESSAGE_VERSION: u16 = 1;

/// @notice Address of the L2CrossDomainMessenger contract on Base Sepolia testnet.
///
/// @dev This is the canonical cross-domain messenger contract that handles message passing between L1 and L2. All
///      cross-chain messages must be validated against this trusted contract address to ensure message authenticity.
///      Contract address: 0x2c85Bb93B4c1F07E80a242FfB3Fa9c0e8b72BB00
#[constant]
pub const REMOTE_MESSENGER: [u8; 20] = hex!("2c85Bb93B4c1F07E80a242FfB3Fa9c0e8b72BB00");

/// @notice Address of the L2StandardBridge contract on Base Sepolia testnet.
///
/// @dev This is the canonical bridge contract that handles token transfers between L1 and L2. All bridge operations
///      must be validated against this trusted contract address to ensure the authenticity of deposit and withdrawal
///      operations. Contract address: 0xC7ae1af5aFd9ED2E65495BFdF4639FbDB3a2ab57
#[constant]
pub const REMOTE_BRIDGE: [u8; 20] = hex!("C7ae1af5aFd9ED2E65495BFdF4639FbDB3a2ab57");

/// @notice Default sender address used when no specific sender is provided.
///
/// @dev This is a well-known "dead" address (0x000000000000000000000000000000000000dEaD) commonly used in blockchain
///      systems as a placeholder when no valid sender exists. It's intentionally an address that no one controls.
#[constant]
pub const DEFAULT_MESSENGER_CALLER: [u8; 20] = hex!("000000000000000000000000000000000000dEaD");

/// @notice Base constant overhead gas cost added to every relayed message.
///
/// @dev This covers the fixed computational cost of processing a message regardless of its content, including
///      signature verification, state updates, and basic message validation. Set to 200,000 gas units based on
///      empirical testing.
#[constant]
pub const RELAY_CONSTANT_OVERHEAD: u64 = 200_000;

/// @notice Gas reserved specifically for executing the external call in `relayMessage`.
///
/// @dev This gas allocation ensures that the actual target contract call has sufficient gas to execute. The 40,000 gas
///      limit provides a reasonable buffer for most standard contract interactions while preventing excessive gas
///      consumption.
#[constant]
pub const RELAY_CALL_OVERHEAD: u64 = 40_000;

/// @notice Gas reserved for finalizing message execution after the external call completes.
///
/// @dev This covers post-execution operations such as updating message status, emitting events, and performing cleanup
///      operations. The 40,000 gas allocation ensures these critical finalization steps can always complete
///      successfully.
#[constant]
pub const RELAY_RESERVED_GAS: u64 = 40_000;

/// @notice Gas buffer allocated between the `hasMinGas` check and the external call.
///
/// @dev This small buffer (5,000 gas) accounts for gas consumption that occurs between validating minimum gas
///      requirements and actually executing the external call. Prevents edge cases where gas validation passes but
///      execution fails due to intermediate gas consumption.
#[constant]
pub const RELAY_GAS_CHECK_BUFFER: u64 = 5_000;

/// @notice Numerator for calculating dynamic gas overhead based on message size.
///
/// @dev Used in the formula: (baseGas * numerator) / denominator to calculate additional gas required for larger
///      messages. The ratio 64/63 adds approximately 1.6% overhead, accounting for the increased computational cost of
///      processing larger messages.
#[constant]
pub const MIN_GAS_DYNAMIC_OVERHEAD_NUMERATOR: u64 = 64;

/// @notice Denominator for calculating dynamic gas overhead based on message size.
///
/// @dev Used with MIN_GAS_DYNAMIC_OVERHEAD_NUMERATOR to create a small percentage overhead. The 63/64 ratio is chosen
///      to be minimal but sufficient to cover the marginal cost of processing additional message data.
#[constant]
pub const MIN_GAS_DYNAMIC_OVERHEAD_DENOMINATOR: u64 = 63;

/// @notice Overhead cost for ABI encoding the complete relayMessage call.
///
/// @dev When a message is fully ABI-encoded for the relayMessage function call, this constant represents the additional
///      bytes required for the encoding structure. The value 260 is an upper bound; actual overhead ranges from 228-260
///      bytes depending on message content. This accounts for function selectors, parameter encoding, and data
///      structure overhead.
#[constant]
pub const ENCODING_OVERHEAD: u64 = 260;

/// @notice Base gas cost required for any transaction on Ethereum Virtual Machine.
///
/// @dev This is the fundamental cost of initiating any transaction, regardless of its complexity. The 21,000 gas value
///      is standardized across EVM-compatible networks and covers basic transaction validation and state tree updates.
#[constant]
pub const TX_BASE_GAS: u64 = 21_000;

/// @notice Additional gas cost per byte of non-zero calldata in a message.
///
/// @dev Each byte of calldata costs 16 gas units to process. This cost applies to all non-zero bytes in the transaction
///      data and is used to calculate the minimum gas required for messages based on their data size.
#[constant]
pub const MIN_GAS_CALLDATA_OVERHEAD: u64 = 16;

/// @notice Floor gas cost per byte of non-zero calldata introduced in EIP-7623.
///
/// @dev This higher per-byte cost (40 gas) was introduced to address concerns about cheap data availability attacks. It
///      represents a minimum cost floor for calldata regardless of other optimizations, providing better economic
///      security for the network.
#[constant]
pub const FLOOR_CALLDATA_OVERHEAD: u64 = 40;

/// @notice Seed bytes used for deriving the native SOL vault PDA.
///
/// @dev This seed generates the PDA that holds native SOL tokens in the bridge system. When users deposit SOL for
///      bridging to Base, the SOL is stored in this vault account until withdrawal.
#[constant]
pub const SOL_VAULT_SEED: &[u8] = b"sol_vault";

/// @notice Seed bytes used for deriving SPL token vault PDAs.
///
/// @dev This seed generates PDAs that hold SPL tokens during bridge operations. Each bridged SPL token has its own
///      vault derived from this seed plus token-specific data. These vaults store tokens that users deposit for
///      bridging to L1, ensuring secure custody until withdrawal or bridging completion.
#[constant]
pub const TOKEN_VAULT_SEED: &[u8] = b"token_vault";

/// @notice Seed bytes used for deriving the messenger state PDA.
///
/// @dev This seed generates the PDA that stores the cross-chain messenger state, including message tracking, validation
///      data, and protocol configuration. The PDA pattern ensures state integrity and program-controlled access.
#[constant]
pub const MESSENGER_SEED: &[u8] = b"messenger";

/// @notice Seed bytes used for deriving output root PDA.
///
/// @dev Output roots represent the MMR root of L2 state at specific block heights. This PDA stores validated output
///      roots from the L2 that are used to prove the validity of withdrawal transactions and other L2 state claims.
#[constant]
pub const OUTPUT_ROOT_SEED: &[u8] = b"output_root";

/// @notice Seed bytes used for deriving individual message PDA.
///
/// @dev Each cross-chain message gets its own PDA derived from this seed plus message-specific data. This allows the
///      program to track message status, prevent replay attacks, and manage message lifecycle independently.
#[constant]
pub const MESSAGE_SEED: &[u8] = b"message";

/// @notice Seed bytes used for deriving mint authority PDA.
///
/// @dev The mint PDA controls the minting and burning of bridged tokens on Solana. This PDA serves as the mint
///      authority for SPL tokens that represent L1 assets, ensuring only the bridge program can mint/burn tokens during
///      bridge operations.
#[constant]
pub const MINT_SEED: &[u8] = b"mint";

/// @notice Seed bytes used for deriving the main bridge state PDA.
///
/// @dev This seed generates the PDA that stores the primary bridge configuration and state data. This includes bridge
///      settings, administrative controls, and global bridge parameters.
#[constant]
pub const BRIDGE_SEED: &[u8] = b"bridge";

/// @notice Public key representing the native SOL token in the bridge system.
///
/// @dev This special identifier is used to distinguish native SOL transfers from SPL token transfers in bridge
///      operations. When this pubkey is used as the token identifier, the bridge handles native SOL instead of minted
///      SPL tokens.
#[constant]
pub const NATIVE_SOL_PUBKEY: Pubkey = pubkey!("SoL1111111111111111111111111111111111111111");

/// @notice Public key of the trusted oracle for validating L2 state commitments.
///
/// @dev This oracle is responsible for submitting and validating L2 output roots and other critical state information.
///      The oracle must be highly trusted as it can influence withdrawal validations and cross-chain message
///      authenticity.
#[constant]
// pub const TRUSTED_ORACLE: Pubkey = pubkey!("eEwCrQLBdQchykrkYitkYUZskd7MPrU2YxBXcPDPnMt"); // un-comment for Devnet deployments
pub const TRUSTED_ORACLE: Pubkey = pubkey!("H4BF4JEUcLaNTEp4ppU5YBx8buWfQKnp32UMBH25Rp2V"); // for local testing

/// @notice Program ID for the SPL Token program.
///
/// @dev This is the standard Solana program for managing fungible tokens (SPL tokens). The bridge interacts with this
///      program to mint, burn, and transfer tokens during cross-chain operations. All bridged ERC-20 tokens and ETH
///      become SPL tokens.
#[constant]
pub const TOKEN_PROGRAM_ID: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

/// @notice Function selector for the `finalizeBridgeERC20` function on Base.
///
/// @dev This 4-byte selector (0x2d916920) identifies the specific function call used to finalize ERC-20 token
///      withdrawals on the Base bridge contract. It's used when constructing cross-chain messages for token withdrawal
///      completion.
#[constant]
pub const FINALIZE_BRIDGE_TOKEN_SELECTOR: [u8; 4] = hex!("2d916920");

/// @notice Function selector for the `relayMessage` function on Base.
///
/// @dev This 4-byte selector (0x54aa43a3) identifies the specific function call used to relay cross-chain messages
///      on the Base messenger contract. It's used when constructing Base transaction data for message execution and
///      validation.
#[constant]
pub const RELAY_MESSAGE_SELECTOR: [u8; 4] = hex!("54aa43a3");

/// @notice Gas cost per byte of transaction data on Ethereum.
///
/// @dev Used for calculating the L1 gas cost of including transaction data. Each byte of data in an Ethereum
///      transaction costs 40 gas units to process, which is factored into cross-chain message cost calculations.
#[constant]
pub const GAS_PER_BYTE_COST: u64 = 40;

/// @notice Base transaction cost on Ethereum for any transaction.
///
/// @dev Every Ethereum transaction has a minimum cost of 21,000 gas regardless of its complexity. This base cost covers
///      transaction validation, account updates, and inclusion in the blockchain state.
#[constant]
pub const BASE_TRANSACTION_COST: u64 = 21000;

/// @notice Conversion factor between SOL and ETH for gas fee calculations.
///
/// @dev This factor is used to convert SOL-denominated gas costs to ETH-equivalent values for cross-chain operations.
///      The factor of 15 represents the approximate price ratio, though actual conversion should account for current
///      market rates. Used primarily for estimating Base gas costs in SOL terms.
#[constant]
pub const SOL_TO_ETH_FACTOR: u64 = 15;

/// @notice Public key of the account that receives gas fees from bridge operations.
///
/// @dev This account collects gas fees charged for cross-chain message relaying and bridge operations. Gas fees help
///      cover the cost of Base transaction execution and provide economic sustainability for the bridge infrastructure.
///      The fees are used to compensate for Base gas costs incurred during message finalization.
#[constant]
pub const GAS_FEE_RECEIVER: Pubkey = pubkey!("H4BF4JEUcLaNTEp4ppU5YBx8buWfQKnp32UMBH25Rp2V");
