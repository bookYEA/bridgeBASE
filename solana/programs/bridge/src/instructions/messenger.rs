use crate::{
    BridgePayload, Ix, Message, Messenger, DEFAULT_MESSENGER_CALLER, ENCODING_OVERHEAD,
    FLOOR_CALLDATA_OVERHEAD, GAS_FEE_RECEIVER, MESSAGE_VERSION, MESSENGER_SEED,
    MIN_GAS_CALLDATA_OVERHEAD, MIN_GAS_DYNAMIC_OVERHEAD_DENOMINATOR,
    MIN_GAS_DYNAMIC_OVERHEAD_NUMERATOR, RELAY_CALL_OVERHEAD, RELAY_CONSTANT_OVERHEAD,
    RELAY_GAS_CHECK_BUFFER, RELAY_MESSAGE_SELECTOR, RELAY_RESERVED_GAS, REMOTE_MESSENGER,
    TX_BASE_GAS,
};
use anchor_lang::solana_program::keccak;
use anchor_lang::{prelude::*, solana_program};
use std::cmp::max;

use super::{portal, token_bridge};

/// Account structure for sending cross-chain messages from Solana to Base
///
/// This struct defines the accounts required for the send_message instruction.
#[derive(Accounts)]
pub struct SendMessage<'info> {
    // Portal accounts
    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK: This is the hardcoded gas fee receiver account.
    #[account(mut, address = GAS_FEE_RECEIVER)]
    pub gas_fee_receiver: AccountInfo<'info>,

    pub system_program: Program<'info, System>,

    // Messenger accounts
    #[account(mut, seeds = [MESSENGER_SEED], bump = messenger.bump)]
    pub messenger: Account<'info, Messenger>,
}

/// Emitted whenever a message is sent to Base
///
/// This event provides all necessary information to reconstruct and relay the message on Base.
#[event]
pub struct SentMessage {
    /// Address of the recipient contract on Base
    pub target: [u8; 20],
    /// Solana public key of the account that sent the message
    pub sender: Pubkey,
    /// The message data to be executed on Base
    pub message: Vec<u8>,
    /// Unique versioned nonce attached to the message for replay protection
    pub message_nonce: [u8; 32],
    /// Native value (in lamports) sent along with the message to the recipient
    pub value: u64,
    /// Minimum gas limit that the message must be executed with on Base
    pub gas_limit: u64,
}

/// Emitted whenever a message from Base is successfully relayed on Solana
#[event]
pub struct RelayedMessage {
    /// Keccak256 hash of the message that was successfully relayed
    pub msg_hash: [u8; 32],
}

/// Emitted whenever a message from Base fails to be relayed on Solana
///
/// Failed messages can be retried later using the relay mechanism.
#[event]
pub struct FailedRelayedMessage {
    /// Keccak256 hash of the message that failed to be relayed
    pub msg_hash: [u8; 32],
}

/// Sends a message to some target address on Base
///
/// Note that if the call always reverts, then the message will be unrelayable, and any SOL sent will be
/// permanently locked. The same will occur if the target on Base is considered unsafe.
///
/// # Arguments
/// * `ctx`           - The transaction context containing accounts
/// * `target`        - Target contract or wallet address on Base
/// * `message`       - Message data to trigger the target address with
/// * `min_gas_limit` - Minimum gas limit that the message can be executed with on Base
pub fn send_message_handler(
    ctx: Context<SendMessage>,
    target: [u8; 20],
    message: Vec<u8>,
    min_gas_limit: u32,
) -> Result<()> {
    send_message_internal(
        &ctx.accounts.system_program,
        &ctx.accounts.user,
        &ctx.accounts.gas_fee_receiver,
        &mut ctx.accounts.messenger,
        ctx.accounts.user.key(),
        target,
        message,
        min_gas_limit,
    )
}

/// Payload structure for messages being relayed from Base to Solana
///
/// This struct contains the essential components of a cross-chain message for deserialization.
#[derive(AnchorDeserialize)]
pub struct MessengerPayload {
    /// The unique nonce of the message, including version information
    pub nonce: [u8; 32],
    /// The address of the account that initiated the message on Base
    pub messenger_caller: [u8; 20],
    /// The message data to be executed on Solana
    pub message: Vec<u8>,
}

/// Relays a message that was sent by the remote CrossChainMessenger contract
///
/// Can only be executed via cross-chain call from the remote messenger OR if the message was
/// already received once and is currently being replayed after a previous failure.
///
/// # Arguments
/// * `message`                 - The message account that tracks the relay state and prevents double execution
/// * `remaining_accounts`      - Additional accounts required for executing the message instructions
/// * `messenger_payload`       - The decoded message payload containing nonce, caller, and message data
/// * `is_called_from_receiver` - Whether this call originates from the message receiver (vs replay)
pub fn relay_message<'info>(
    message: &mut Account<'info, Message>,
    remaining_accounts: &'info [AccountInfo<'info>],
    messenger_payload: MessengerPayload,
    is_called_from_receiver: bool,
) -> Result<()> {
    // On L1 this function will check the Portal for its paused status.
    // On L2 this function should be a no-op, because paused will always return false.
    require!(!paused(), MessengerError::BridgeIsPaused);

    // We use the v1 message hash as the unique identifier for the message because it commits
    // to the value and minimum gas limit of the message.
    // TODO: Fix this, it should be linked with the message account.
    let versioned_hash = hash_message(&messenger_payload);

    if is_called_from_receiver && message.message_passer_caller == REMOTE_MESSENGER {
        // These properties should always hold when the message is first submitted (as
        // opposed to being replayed).
        require!(
            !message.failed_message,
            MessengerError::CannotBeFailedMessage
        );
    } else {
        require!(
            message.failed_message,
            MessengerError::CanOnlyRetryAFailedMessage
        );
    }

    require!(
        !message.successful_message,
        MessengerError::MessageHasAlreadyBeenRelayed
    );

    message.messenger_caller = messenger_payload.messenger_caller;
    let success = handle_ixs(remaining_accounts, message, &messenger_payload.message);
    message.messenger_caller = DEFAULT_MESSENGER_CALLER;

    if success == Ok(()) {
        message.successful_message = true;
        emit!(RelayedMessage {
            msg_hash: versioned_hash
        });
    } else {
        message.failed_message = true;

        emit!(FailedRelayedMessage {
            msg_hash: versioned_hash
        })
    }

    Ok(())
}

/// Internal function to send a cross-chain message with proper gas calculation and fee handling
///
/// This function handles the core logic of message sending including nonce management,
/// gas calculation, and encoding the relay call for Base.
///
/// # Arguments
/// * `system_program`   - The Solana system program for SOL transfers
/// * `gas_fee_payer`    - The account that will pay the gas fees for the cross-chain transaction
/// * `gas_fee_receiver` - The account that receives the gas fees
/// * `messenger`        - The messenger state account for nonce tracking
/// * `from`             - The Solana public key of the message sender
/// * `target`           - The target address on Base
/// * `message`          - The message data to be executed on Base
/// * `min_gas_limit`    - The minimum gas limit for executing the message on Base
#[allow(clippy::too_many_arguments)]
pub fn send_message_internal<'info>(
    system_program: &Program<'info, System>,
    gas_fee_payer: &Signer<'info>,
    gas_fee_receiver: &AccountInfo<'info>,
    messenger: &mut Account<'info, Messenger>,
    from: Pubkey,
    target: [u8; 20],
    message: Vec<u8>,
    min_gas_limit: u32,
) -> Result<()> {
    let message_nonce = encode_versioned_nonce(messenger.msg_nonce, MESSAGE_VERSION);

    // Triggers a message to the remote messenger. Note that the amount of gas provided to the
    // message is the amount of gas requested by the user PLUS the base gas value. We want to
    // guarantee the property that the call to the target contract will always have at least
    // the minimum gas limit specified by the user.
    send_message(
        system_program,
        gas_fee_payer,
        gas_fee_receiver,
        REMOTE_MESSENGER,
        base_gas(message.len() as u64, min_gas_limit),
        &encode_relay_message_call(message_nonce, from, target, 0, min_gas_limit, &message),
    )?;

    emit!(SentMessage {
        target,
        sender: from,
        message,
        message_nonce,
        value: 0,
        gas_limit: min_gas_limit as u64,
    });

    messenger.msg_nonce += 1;

    Ok(())
}

/// Sends a low-level message to the remote messenger via the portal
///
/// This function creates a deposit transaction that will be relayed to Base.
///
/// # Arguments
/// * `system_program`   - The Solana system program for SOL transfers
/// * `gas_fee_payer`    - The account that will pay the gas fees for the cross-chain transaction
/// * `gas_fee_receiver` - The account that receives the gas fees
/// * `to`               - Recipient address of the message on Base
/// * `gas_limit`        - Minimum gas limit the message can be executed with on Base
/// * `data`             - Encoded message data to be sent to Base
fn send_message<'info>(
    system_program: &Program<'info, System>,
    gas_fee_payer: &Signer<'info>,
    gas_fee_receiver: &AccountInfo<'info>,
    to: [u8; 20],
    gas_limit: u64,
    data: &[u8],
) -> Result<()> {
    portal::deposit_transaction_internal(
        system_program,
        gas_fee_payer,
        gas_fee_receiver,
        local_messenger_pubkey(),
        to,
        gas_limit,
        false,
        data,
    )
}

/// Computes the deterministic public key for the local messenger account
///
/// This function generates a public key by hashing the program ID with the messenger seed,
/// which is equivalent to keccak256(abi.encodePacked(programId, "messenger")) in Solidity.
pub fn local_messenger_pubkey() -> Pubkey {
    // Equivalent to keccak256(abi.encodePacked(programId, "messenger"));
    let mut data_to_hash = Vec::new();
    data_to_hash.extend_from_slice(crate::ID.as_ref());
    data_to_hash.extend_from_slice(MESSENGER_SEED);
    let hash = keccak::hash(&data_to_hash);
    Pubkey::new_from_array(hash.to_bytes())
}

/// Computes the amount of gas required to guarantee that a given message will be
/// received on Base without running out of gas
///
/// Guaranteeing that a message will not run out of gas is important because this ensures
/// that a message can always be replayed on Base if it fails to execute completely.
/// The calculation includes various overhead costs and follows EIP-150 and EIP-7623 standards.
///
/// # Arguments
/// * `message_len`   - Length of the message data to compute the required gas for
/// * `min_gas_limit` - Minimum desired gas limit when message goes to target on remote chain
fn base_gas(message_len: u64, min_gas_limit: u32) -> u64 {
    // Base gas should really be computed on the fully encoded message but that would break the
    // expected API, so we instead just add the encoding overhead to the message length inside
    // of this function.

    // We need a minimum amount of execution gas to ensure that the message will be received on
    // the remote side without running out of gas (stored within the failedMessages mapping).
    // If we get beyond the hasMinGas check, then we *must* supply more than minGasLimit to
    // the external call.
    let execution_gas = RELAY_CONSTANT_OVERHEAD // Constant costs for relayMessage
        // Covers dynamic parts of the CALL opcode
        + RELAY_CALL_OVERHEAD
        // Ensures execution of relayMessage completes after call
        + RELAY_RESERVED_GAS
        // Buffer between hasMinGas check and the CALL
        + RELAY_GAS_CHECK_BUFFER
        // Minimum gas limit, multiplied by 64/63 to account for EIP-150.
        + ((min_gas_limit as u64 * MIN_GAS_DYNAMIC_OVERHEAD_NUMERATOR)
            / MIN_GAS_DYNAMIC_OVERHEAD_DENOMINATOR);

    // Total message size is the result of properly ABI encoding the call to relayMessage.
    // Since we only get the message data and not the rest of the calldata, we use the
    // ENCODING_OVERHEAD constant to conservatively account for the remaining bytes.
    let total_message_size = message_len + ENCODING_OVERHEAD;

    // Finally, replicate the transaction cost formula as defined after EIP-7623. This is
    // mostly relevant in the SOL -> Base case because we need to be able to cover the intrinsic
    // cost of the message but it doesn't hurt in the Base -> SOL case. After EIP-7623, the cost
    // of a transaction is floored by its calldata size. We don't need to account for the
    // contract creation case because this is always a call to relayMessage.
    TX_BASE_GAS
        + max(
            execution_gas + (total_message_size * MIN_GAS_CALLDATA_OVERHEAD),
            total_message_size * FLOOR_CALLDATA_OVERHEAD,
        )
}

/// Encodes a call to the relayMessage function on the remote CrossDomainMessenger contract
///
/// This function creates ABI-encoded calldata for the relayMessage function with the signature:
/// relayMessage(bytes32,address,address,uint256,uint256,bytes)
///
/// # Arguments
/// * `nonce`         - The versioned nonce of the message
/// * `sender`        - The Solana public key of the original message sender
/// * `target`        - The target contract address on Base
/// * `value`         - The native value to send with the message (always 0 for Solana messages)
/// * `min_gas_limit` - The minimum gas limit for executing the message on Base
/// * `message`       - The message data to be executed on Base
fn encode_relay_message_call(
    nonce: [u8; 32],
    sender: Pubkey,
    target: [u8; 20],
    value: u64,
    min_gas_limit: u32,
    message: &[u8],
) -> Vec<u8> {
    // Create a vector to hold the encoded data
    let mut encoded = Vec::new();

    // Add selector for Base.CrossChainMessenger.relayMessage(bytes32,address,address,uint256,uint256,bytes)
    encoded.extend_from_slice(&RELAY_MESSAGE_SELECTOR);

    // Add nonce (32 bytes) - nonce is already 32 bytes
    encoded.extend_from_slice(&nonce);

    // Add sender (32 bytes) - Pubkey is already 32 bytes (Solana Pubkey passed as 32-byte value)
    encoded.extend_from_slice(sender.as_ref());

    // Add target (32 bytes) - pad 20-byte address to 32 bytes
    let mut target_bytes = [0u8; 32];
    target_bytes[12..32].copy_from_slice(&target);
    encoded.extend_from_slice(&target_bytes);

    // Add value (32 bytes) - pad u64 to 32 bytes
    let mut value_bytes = [0u8; 32];
    value_bytes[24..32].copy_from_slice(&value.to_be_bytes());
    encoded.extend_from_slice(&value_bytes);

    // Add min_gas_limit (32 bytes) - pad u32 to 32 bytes
    let mut gas_bytes = [0u8; 32];
    gas_bytes[28..32].copy_from_slice(&min_gas_limit.to_be_bytes());
    encoded.extend_from_slice(&gas_bytes);

    // Add message length and data (dynamic type)
    // First add offset to message data (32 bytes)
    // Offset is 6 * 32 = 192 bytes (for the 6 preceding static parameters of 32 bytes each)
    let mut offset_bytes = [0u8; 32];
    offset_bytes[24..32].copy_from_slice(&(192u64).to_be_bytes());
    encoded.extend_from_slice(&offset_bytes);

    // Add message length (32 bytes)
    let mut length_bytes = [0u8; 32];
    length_bytes[24..32].copy_from_slice(&(message.len() as u64).to_be_bytes());
    encoded.extend_from_slice(&length_bytes);

    // Add message data
    encoded.extend_from_slice(message);

    // Pad message data to multiple of 32 bytes
    let padding_bytes = (32 - (message.len() % 32)) % 32;
    encoded.extend_from_slice(&vec![0u8; padding_bytes]);

    encoded
}

/// Adds a version number into the first two bytes of a message nonce
///
/// This encoding allows for future protocol upgrades while maintaining backwards compatibility.
/// The version is stored in the first 2 bytes, and the nonce in the lower 8 bytes for EVM compatibility.
///
/// # Arguments
/// * `nonce`   - Message nonce to encode into
/// * `version` - Version number to encode into the message nonce
fn encode_versioned_nonce(nonce: u64, version: u16) -> [u8; 32] {
    let mut nonce_bytes = [0u8; 32];
    nonce_bytes[0..2].copy_from_slice(&version.to_be_bytes());
    nonce_bytes[24..32].copy_from_slice(&nonce.to_be_bytes()); // Store nonce in the lower bytes for EVM compatibility
    nonce_bytes
}

/// Computes the keccak256 hash of a messenger payload for message identification
///
/// This hash serves as a unique identifier for cross-chain messages and is used
/// for tracking message relay status and preventing replay attacks.
///
/// # Arguments
/// * `messenger_payload` - The messenger payload containing nonce, caller, and message data
fn hash_message(messenger_payload: &MessengerPayload) -> [u8; 32] {
    let mut data = Vec::new();
    data.extend_from_slice(&messenger_payload.nonce);
    data.extend_from_slice(&messenger_payload.messenger_caller);
    data.extend_from_slice(&messenger_payload.message);
    keccak::hash(&data).0
}

/// Handles the execution of instructions contained within a cross-chain message
///
/// This function deserializes the message data into Solana instructions and executes them.
/// Special handling is provided for bridge token operations, while other instructions
/// are executed using the standard Solana program invocation mechanism.
///
/// # Arguments
/// * `remaining_accounts` - Additional accounts required for executing the message instructions
/// * `message`            - The message account for tracking execution state and caller context
/// * `message_data`       - The serialized instruction data to be executed
fn handle_ixs<'info>(
    remaining_accounts: &'info [AccountInfo<'info>],
    message: &mut Account<'info, Message>,
    message_data: &[u8],
) -> Result<()> {
    let ixs_vec = Vec::<Ix>::try_from_slice(message_data)?;
    for ix in &ixs_vec {
        if ix.program_id == token_bridge::local_bridge_pubkey() {
            token_bridge::finalize_bridge_tokens(
                message,
                remaining_accounts,
                BridgePayload::try_from_slice(&ix.data)?,
            )?;
        } else {
            solana_program::program::invoke(&ix.into(), remaining_accounts)?;
        }
    }

    Ok(())
}

/// Error codes for cross-chain messenger operations
#[error_code]
pub enum MessengerError {
    /// Thrown when attempting to relay messages while the bridge is paused for security
    #[msg("Bridge is paused")]
    BridgeIsPaused,
    /// Thrown when attempting to replay a message that has not previously failed
    #[msg("Cannot be failed message")]
    CannotBeFailedMessage,
    /// Thrown when attempting to retry a message that has not been marked as failed
    #[msg("Can only retry a failed message")]
    CanOnlyRetryAFailedMessage,
    /// Thrown when attempting to relay a message that has already been successfully executed
    #[msg("Message has already been relayed")]
    MessageHasAlreadyBeenRelayed,
}

/// Returns the paused status of the bridge system
pub fn paused() -> bool {
    false
}
