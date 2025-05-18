use crate::{
    constants::OTHER_MESSENGER, ENCODING_OVERHEAD, FLOOR_CALLDATA_OVERHEAD, MESSAGE_VERSION,
    MESSENGER_SEED, MIN_GAS_CALLDATA_OVERHEAD, MIN_GAS_DYNAMIC_OVERHEAD_DENOMINATOR,
    MIN_GAS_DYNAMIC_OVERHEAD_NUMERATOR, RELAY_CALL_OVERHEAD, RELAY_CONSTANT_OVERHEAD,
    RELAY_GAS_CHECK_BUFFER, RELAY_RESERVED_GAS, TX_BASE_GAS,
};
use crate::{BridgePayload, Ix, Message, MessengerPayload, DEFAULT_SENDER};
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::keccak;
use anchor_lang::{prelude::*, solana_program};
use hex_literal::hex;
use std::cmp::max;

use super::{portal, standard_bridge};

#[derive(Accounts)]
pub struct SendMessage<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut, seeds = [MESSENGER_SEED], bump)]
    pub msg_state: Account<'info, Messenger>,
}

#[derive(InitSpace)]
#[account]
pub struct Messenger {
    pub msg_nonce: u64,
}

#[event]
// Emitted whenever a message is sent to the other chain.
pub struct SentMessage {
    pub target: [u8; 20],        // Address of the recipient of the message.
    pub sender: Pubkey,          // Address of the sender of the message.
    pub message: Vec<u8>,        // Message to trigger the recipient address with.
    pub message_nonce: [u8; 32], // Unique nonce attached to the message.
    pub value: u64,              // Native value sent along with the message to the recipient.
    pub gas_limit: u64,          // Minimum gas limit that the message can be executed with.
}

#[event]
// Emitted whenever a message is successfully relayed on this chain.
pub struct RelayedMessage {
    pub msg_hash: [u8; 32], // Hash of the message that was relayed.
}

#[event]
// Emitted whenever a message fails to be relayed on this chain.
pub struct FailedRelayedMessage {
    pub msg_hash: [u8; 32], // Hash of the message that failed to be relayed.
}

/// @notice Sends a message to some target address on Base. Note that if the call
///         always reverts, then the message will be unrelayable, and any SOL sent will be
///         permanently locked. The same will occur if the target on the other chain is
///         considered unsafe (see the _isUnsafeTarget() function).
/// @param _target      Target contract or wallet address.
/// @param _message     Message to trigger the target address with.
/// @param _minGasLimit Minimum gas limit that the message can be executed with.
pub fn send_message_handler(
    ctx: Context<SendMessage>,
    target: [u8; 20],
    message: Vec<u8>,
    min_gas_limit: u32,
) -> Result<()> {
    send_message_internal(
        &mut ctx.accounts.msg_state,
        ctx.accounts.user.key(),
        target,
        message,
        min_gas_limit,
    )
}

/// @notice Relays a message that was sent by the other CrossDomainMessenger contract. Can only
///         be executed via cross-chain call from the other messenger OR if the message was
///         already received once and is currently being replayed.
/// @param _nonce       Nonce of the message being relayed.
/// @param _sender      Address of the user who sent the message.
/// @param _message     Message to send to the target.
pub fn relay_message<'info>(
    message_account: &mut Account<'info, Message>,
    vault: &AccountInfo<'info>,
    account_infos: &'info [AccountInfo<'info>],
    remote_sender: &[u8; 20],
    msg: MessengerPayload,
) -> Result<()> {
    // On L1 this function will check the Portal for its paused status.
    // On L2 this function should be a no-op, because paused will always return false.
    if paused() {
        return err!(MessengerError::BridgeIsPaused);
    }

    // We use the v1 message hash as the unique identifier for the message because it commits
    // to the value and minimum gas limit of the message.
    let versioned_hash = hash_message(msg.nonce, msg.sender, &msg.message);

    if remote_sender == &OTHER_MESSENGER {
        // These properties should always hold when the message is first submitted (as
        // opposed to being replayed).
        if message_account.failed_message {
            return err!(MessengerError::CannotBeFailedMessage);
        }
    } else {
        if !message_account.failed_message {
            return err!(MessengerError::CanOnlyRetryAFailedMessage);
        }
    }

    if message_account.successful_message {
        return err!(MessengerError::MessageHasAlreadyBeenRelayed);
    }

    message_account.sender = msg.sender;
    let success = handle_ixs(account_infos, message_account, vault, &msg.message);
    message_account.sender = DEFAULT_SENDER;

    if success == Ok(()) {
        message_account.successful_message = true;
        emit!(RelayedMessage {
            msg_hash: versioned_hash
        });
    } else {
        message_account.failed_message = true;
        emit!(FailedRelayedMessage {
            msg_hash: versioned_hash
        })
    }

    Ok(())
}

pub fn send_message_internal<'info>(
    msg_state: &mut Account<'info, Messenger>,
    from: Pubkey,
    target: [u8; 20],
    message: Vec<u8>,
    min_gas_limit: u32,
) -> Result<()> {
    // Triggers a message to the other messenger. Note that the amount of gas provided to the
    // message is the amount of gas requested by the user PLUS the base gas value. We want to
    // guarantee the property that the call to the target contract will always have at least
    // the minimum gas limit specified by the user.
    send_message(
        OTHER_MESSENGER,
        base_gas(message.len() as u64, min_gas_limit),
        encode_with_selector(
            message_nonce(msg_state.msg_nonce),
            from,
            target,
            0,
            min_gas_limit,
            message.clone(),
        ),
    )?;

    emit!(SentMessage {
        target,
        sender: from,
        message,
        message_nonce: message_nonce(msg_state.msg_nonce),
        value: 0,
        gas_limit: min_gas_limit as u64,
    });

    msg_state.msg_nonce += 1;

    Ok(())
}

/// @notice Sends a low-level message to the other messenger.
///
/// @param _to       Recipient of the message on the other chain.
/// @param _gasLimit Minimum gas limit the message can be executed with.
/// @param _data     Message data.
fn send_message<'info>(to: [u8; 20], gas_limit: u64, data: Vec<u8>) -> Result<()> {
    portal::deposit_transaction_internal(local_messenger_pubkey(), to, gas_limit, false, data)
}

pub fn local_messenger_pubkey() -> Pubkey {
    // Equivalent to keccak256(abi.encodePacked(programId, "messenger"));
    let mut data_to_hash = Vec::new();
    data_to_hash.extend_from_slice(crate::ID.as_ref());
    data_to_hash.extend_from_slice(b"messenger");
    let hash = keccak::hash(&data_to_hash);
    return Pubkey::new_from_array(hash.to_bytes());
}

/// @notice Computes the amount of gas required to guarantee that a given message will be
///         received on Base without running out of gas. Guaranteeing that a message will
///         not run out of gas is important because this ensures that a message can always
///         be replayed on the other chain if it fails to execute completely.
/// @param message_len  Length of message to compute the amount of required gas for.
/// @param _minGasLimit Minimum desired gas limit when message goes to target.
/// @return Amount of gas required to guarantee message receipt.
fn base_gas(message_len: u64, min_gas_limit: u32) -> u64 {
    // Base gas should really be computed on the fully encoded message but that would break the
    // expected API, so we instead just add the encoding overhead to the message length inside
    // of this function.

    // We need a minimum amount of execution gas to ensure that the message will be received on
    // the other side without running out of gas (stored within the failedMessages mapping).
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
    return TX_BASE_GAS
        + max(
            execution_gas + (total_message_size * MIN_GAS_CALLDATA_OVERHEAD),
            total_message_size * FLOOR_CALLDATA_OVERHEAD,
        );
}

fn encode_with_selector(
    nonce: [u8; 32],
    sender: Pubkey,
    target: [u8; 20],
    value: u64,
    min_gas_limit: u32,
    message: Vec<u8>,
) -> Vec<u8> {
    // Create a vector to hold the encoded data
    let mut encoded = Vec::new();

    // Add selector for Base.CrossChainMessenger.relayMessage(bytes32,address,address,uint256,uint256,bytes)
    // Selector: 0x54aa43a3 (first 4 bytes of keccak256 hash of the function signature)
    encoded.extend_from_slice(&hex!("54aa43a3"));

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
    encoded.extend_from_slice(&message);

    // Pad message data to multiple of 32 bytes
    let padding_bytes = (32 - (message.len() % 32)) % 32;
    encoded.extend_from_slice(&vec![0u8; padding_bytes]);

    return encoded;
}

/// @notice Retrieves the next message nonce. Message version will be added to the upper two
///         bytes of the message nonce. Message version allows us to treat messages as having
///         different structures.
/// @return Nonce of the next message to be sent, with added message version.
fn message_nonce(msg_nonce: u64) -> [u8; 32] {
    return encode_versioned_nonce(msg_nonce, MESSAGE_VERSION);
}

/// @notice Adds a version number into the first two bytes of a message nonce.
/// @param _nonce   Message nonce to encode into.
/// @param _version Version number to encode into the message nonce.
/// @return Message nonce with version encoded into the first two bytes.
fn encode_versioned_nonce(nonce: u64, version: u16) -> [u8; 32] {
    let mut nonce_bytes = [0u8; 32];
    nonce_bytes[0..2].copy_from_slice(&version.to_be_bytes());
    nonce_bytes[24..32].copy_from_slice(&nonce.to_be_bytes()); // Store nonce in the lower bytes for EVM compatibility
    return nonce_bytes;
}

fn hash_message(nonce: [u8; 32], sender: [u8; 20], message: &Vec<u8>) -> [u8; 32] {
    let mut data = Vec::new();

    data.extend_from_slice(&nonce);
    data.extend_from_slice(&sender);
    data.extend_from_slice(message);

    return keccak::hash(&data).0;
}

fn handle_ixs<'info>(
    account_infos: &'info [AccountInfo<'info>],
    message_account: &mut Account<'info, Message>,
    vault: &AccountInfo<'info>,
    message: &Vec<u8>,
) -> Result<()> {
    let ixs_vec = Vec::<Ix>::try_from_slice(message)?;
    for ix in &ixs_vec {
        let ix_converted: Instruction = ix.into();
        if ix_converted.program_id == standard_bridge::local_bridge_pubkey() {
            standard_bridge::finalize_bridge_tokens(
                message_account,
                vault,
                account_infos,
                BridgePayload::try_from_slice(&ix_converted.data)?,
            )?;
        } else {
            solana_program::program::invoke(&ix_converted, account_infos)?;
        }
    }

    Ok(())
}

#[error_code]
pub enum MessengerError {
    #[msg("Bridge is paused")]
    BridgeIsPaused,
    #[msg("Cannot be failed message")]
    CannotBeFailedMessage,
    #[msg("Can only retry a failed message")]
    CanOnlyRetryAFailedMessage,
    #[msg("Message has already been relayed")]
    MessageHasAlreadyBeenRelayed,
}

pub fn paused() -> bool {
    return false;
}
