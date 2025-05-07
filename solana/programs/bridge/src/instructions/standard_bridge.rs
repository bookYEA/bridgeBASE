use anchor_lang::prelude::*;
use anchor_lang::solana_program::keccak;
use hex_literal::hex;

use crate::{messenger, MESSENGER_SEED, NATIVE_SOL_PUBKEY, OTHER_BRIDGE, VAULT_SEED};

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
/// @notice Emitted when an SPL or SOL bridge is initiated to Base.
pub struct TokenBridgeInitiated {
    pub local_token: Pubkey, // Address of the token on this chain. Default pubkey signifies SOL.
    pub remote_token: [u8; 20], // Address of the ERC20 on Base.
    pub from: Pubkey,        // Address of the sender.
    pub to: [u8; 20],        // Address of the receiver.
    pub amount: u64,         // Amount of ETH sent.
    pub extra_data: Vec<u8>, // Extra data sent with the transaction.
}

/// @notice Sends SPL tokens or SOL to a receiver's address on Base.
///
/// @param _localToken  Address of the SPL on this chain.
/// @param _remoteToken Address of the corresponding token on Base.
/// @param _to          Address of the receiver.
/// @param _amount      Amount of local tokens to deposit.
/// @param _minGasLimit Minimum amount of gas that the bridge can be relayed with.
/// @param _extraData   Extra data to be sent with the transaction. Note that the recipient will
///                     not be triggered with this data, but it will be emitted and can be used
///                     to identify the transaction.
pub fn bridge_tokens_to_handler(
    ctx: Context<BridgeSolTo>,
    local_token: Pubkey,
    remote_token: [u8; 20],
    to: [u8; 20],
    amount: u64,
    min_gas_limit: u32,
    extra_data: Vec<u8>,
) -> Result<()> {
    let program_id: &[u8] = ctx.program_id.as_ref();
    initiate_bridge_tokens(
        program_id,
        &ctx.accounts.system_program,
        &ctx.accounts.user.to_account_info(),
        &ctx.accounts.vault.to_account_info(),
        &mut ctx.accounts.msg_state,
        ctx.accounts.user.key(),
        local_token,
        remote_token,
        to,
        amount,
        min_gas_limit,
        extra_data,
    )
}

/// @notice Sends SPL tokens or SOL to a receiver's address on Base.
///
/// @param _localToken  Address of the SPL on this chain.
/// @param _remoteToken Address of the corresponding token on Base.
/// @param _to          Address of the receiver.
/// @param _amount      Amount of local tokens to deposit.
/// @param _minGasLimit Minimum amount of gas that the bridge can be relayed with.
/// @param _extraData   Extra data to be sent with the transaction. Note that the recipient will
///                     not be triggered with this data, but it will be emitted and can be used
///                     to identify the transaction.
fn initiate_bridge_tokens<'info>(
    program_id: &[u8],
    system_program: &Program<'info, System>,
    user: &AccountInfo<'info>,
    vault: &AccountInfo<'info>,
    msg_state: &mut Account<'info, Messenger>,
    from: Pubkey,
    local_token: Pubkey,
    remote_token: [u8; 20],
    to: [u8; 20],
    amount: u64,
    min_gas_limit: u32,
    extra_data: Vec<u8>,
) -> Result<()> {
    // Transfer `amount` of local_token from user to vault
    if local_token == NATIVE_SOL_PUBKEY {
        // Transfer lamports from user to vault PDA
        let cpi_context = CpiContext::new(
            system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: user.clone(),
                to: vault.clone(),
            },
        );
        anchor_lang::system_program::transfer(cpi_context, amount)?;
    } else {
        // TODO: implement support for SPL tokens
        return err!(BridgeError::SplTokensNotSupported);
    }

    // TODO: Update stored deposit for `local_token` / `remote_token` pair

    emit!(TokenBridgeInitiated {
        local_token,
        remote_token,
        from,
        to,
        amount,
        extra_data: extra_data.clone()
    });

    // Equivalent to keccak256(abi.encodePacked(programId, "bridge"));
    let mut data_to_hash = Vec::new();
    data_to_hash.extend_from_slice(program_id);
    data_to_hash.extend_from_slice(b"bridge");
    let hash = keccak::hash(&data_to_hash);

    messenger::send_message_internal(
        program_id,
        user,
        msg_state,
        Pubkey::new_from_array(hash.to_bytes()),
        OTHER_BRIDGE,
        encode_with_selector(remote_token, local_token, from, to, amount, extra_data),
        min_gas_limit,
    )
}

fn encode_with_selector(
    remote_token: [u8; 20],
    local_token: Pubkey,
    from: Pubkey,
    to: [u8; 20],
    amount: u64,
    extra_data: Vec<u8>,
) -> Vec<u8> {
    // Create a vector to hold the encoded data
    let mut encoded = Vec::new();

    // Add selector for Base.Bridge.finalizeBridgeToken 0x2d916920 (4 bytes)
    encoded.extend_from_slice(&hex!("2d916920"));

    // Add remote_token (32 bytes) - pad 20-byte address to 32 bytes
    let mut remote_token_bytes = [0u8; 32];
    remote_token_bytes[12..32].copy_from_slice(&remote_token);
    encoded.extend_from_slice(&remote_token_bytes);

    // Add local_token (32 bytes) - Pubkey is already 32 bytes
    encoded.extend_from_slice(local_token.as_ref());

    // Add from (32 bytes) - Pubkey is already 32 bytes
    encoded.extend_from_slice(from.as_ref());

    // Add target (32 bytes) - pad 20-byte address to 32 bytes
    let mut to_bytes = [0u8; 32];
    to_bytes[12..32].copy_from_slice(&to);
    encoded.extend_from_slice(&to_bytes);

    // Add amount (32 bytes) - pad u64 to 32 bytes
    let mut value_bytes = [0u8; 32];
    value_bytes[24..32].copy_from_slice(&amount.to_be_bytes());
    encoded.extend_from_slice(&value_bytes);

    // Add message length and data (dynamic type)
    // First add offset to message data (32 bytes)
    let mut offset_bytes = [0u8; 32];
    // Offset is 6 * 32 = 192 bytes (6 previous parameters of 32 bytes each)
    offset_bytes[31] = 192;
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

    return encoded;
}

#[error_code]
pub enum BridgeError {
    #[msg("SPL Tokens not supported")]
    SplTokensNotSupported,
}
