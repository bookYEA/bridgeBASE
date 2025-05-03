use anchor_lang::prelude::*;

use crate::{messenger, MESSENGER_SEED, OTHER_BRIDGE, VAULT_SEED};

use super::Messenger;

#[derive(Accounts)]
pub struct BridgeSolTo<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK: This is the vault PDA. We are only using it to transfer SOL via CPI
    /// to the system program, so no data checks are required. The address is
    /// verified by the seeds constraint.
    #[account(
        mut,
        seeds = [VAULT_SEED],
        bump
    )]
    pub vault: AccountInfo<'info>,

    #[account(mut, seeds = [MESSENGER_SEED], bump)]
    pub msg_state: Account<'info, Messenger>,

    pub system_program: Program<'info, System>,
}

#[event]
/// @notice Emitted when a SOL bridge is initiated to the other chain.
pub struct SOLBridgeInitiated {
    pub from: Pubkey,        // Address of the sender.
    pub to: [u8; 20],        // Address of the receiver.
    pub amount: u64,         // Amount of ETH sent.
    pub extra_data: Vec<u8>, // Extra data sent with the transaction.
}

/// @notice Sends SOL to a receiver's address on the other chain. Note that if SOL is sent to a
///         smart contract and the call fails, the SOL will be temporarily locked in the
///         StandardBridge on the other chain until the call is replayed. If the call cannot be
///         replayed with any amount of gas (call always reverts), then the SOL will be
///         permanently locked in the StandardBridge on the other chain. SOL will also
///         be locked if the receiver is the other bridge, because finalizeBridgeSOL will revert
///         in that case.
/// @param _to          Address of the receiver.
/// @param _minGasLimit Minimum amount of gas that the bridge can be relayed with.
/// @param _extraData   Extra data to be sent with the transaction. Note that the recipient will
///                     not be triggered with this data, but it will be emitted and can be used
///                     to identify the transaction.
pub fn bridge_sol_to_handler(
    ctx: Context<BridgeSolTo>,
    to: [u8; 20],
    value: u64,
    min_gas_limit: u32,
    extra_data: Vec<u8>,
) -> Result<()> {
    initiate_bridge_sol(
        &ctx.accounts.system_program,
        &ctx.accounts.user.to_account_info(),
        &ctx.accounts.vault.to_account_info(),
        &mut ctx.accounts.msg_state,
        ctx.accounts.user.key(),
        to,
        value,
        min_gas_limit,
        extra_data,
    )
}

/// @notice Initiates a bridge of SOL through the CrossDomainMessenger.
/// @param _from        Address of the sender.
/// @param _to          Address of the receiver.
/// @param _amount      Amount of SOL being bridged.
/// @param _minGasLimit Minimum amount of gas that the bridge can be relayed with.
/// @param _extraData   Extra data to be sent with the transaction. Note that the recipient will
///                     not be triggered with this data, but it will be emitted and can be used
///                     to identify the transaction.
fn initiate_bridge_sol<'info>(
    system_program: &Program<'info, System>,
    user: &AccountInfo<'info>,
    vault: &AccountInfo<'info>,
    msg_state: &mut Account<'info, Messenger>,
    from: Pubkey,
    to: [u8; 20],
    amount: u64,
    min_gas_limit: u32,
    extra_data: Vec<u8>,
) -> Result<()> {
    emit!(SOLBridgeInitiated {
        from,
        to,
        amount,
        extra_data: extra_data.clone()
    });

    messenger::send_message_internal(
        system_program,
        user,
        vault,
        msg_state,
        OTHER_BRIDGE,
        encode_with_selector(from, to, amount, extra_data),
        amount,
        min_gas_limit,
    )
}

fn encode_with_selector(from: Pubkey, to: [u8; 20], amount: u64, extra_data: Vec<u8>) -> Vec<u8> {
    // Create a vector to hold the encoded data
    let mut encoded = Vec::new();

    // Add selector for Base.L2StandardBridge.finalizeBridgeETH 0x1635f5fd (4 bytes)
    encoded.extend_from_slice(&[22, 53, 245, 253]);

    // Add from (32 bytes) - Pubkey is already 32 bytes
    encoded.extend_from_slice(from.as_ref());

    // Add target (32 bytes) - pad 20-byte address to 32 bytes
    let mut target_bytes = [0u8; 32];
    target_bytes[12..32].copy_from_slice(&to);
    encoded.extend_from_slice(&target_bytes);

    // Add amount (32 bytes) - pad u64 to 32 bytes
    let mut value_bytes = [0u8; 32];
    value_bytes[24..32].copy_from_slice(&amount.to_be_bytes());
    encoded.extend_from_slice(&value_bytes);

    // Add message length and data (dynamic type)
    // First add offset to message data (32 bytes)
    let mut offset_bytes = [0u8; 32];
    // Offset is 3 * 32 = 96 bytes (3 previous parameters of 32 bytes each)
    offset_bytes[31] = 96;
    encoded.extend_from_slice(&offset_bytes);

    // Add extra_data length (32 bytes)
    let mut length_bytes = [0u8; 32];
    length_bytes[24..32].copy_from_slice(&(extra_data.len() as u64).to_be_bytes());
    encoded.extend_from_slice(&length_bytes);

    // Add extra data
    encoded.extend_from_slice(&extra_data);

    // Pad extra data to multiple of 32 bytes
    let padding_bytes = (32 - (extra_data.len() % 32)) % 32;
    encoded.extend_from_slice(&vec![0u8; padding_bytes]);

    msg!("actual: {:?}", encoded);

    return encoded;
}
