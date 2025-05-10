use anchor_lang::prelude::*;
use anchor_lang::solana_program::keccak;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};
use hex_literal::hex;

use crate::{messenger, MESSENGER_SEED, NATIVE_SOL_PUBKEY, OTHER_BRIDGE, VAULT_SEED};

use super::Messenger;

#[derive(Accounts)]
pub struct BridgeTokensTo<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK: This is the vault PDA. For SOL, it receives SOL. For SPL, it's the authority for vault_token_account.
    #[account(
        mut,
        seeds = [VAULT_SEED],
        bump
    )]
    pub vault: AccountInfo<'info>,

    #[account(mut, seeds = [MESSENGER_SEED], bump)]
    pub msg_state: Account<'info, Messenger>,

    // SPL Token specific accounts.
    // These accounts must be provided by the client.
    pub mint: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = user,
    )]
    pub from_token_account: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = mint,
        associated_token::authority = vault // Vault PDA is the ATA owner
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

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
/// @param _remoteToken Address of the corresponding token on Base.
/// @param _to          Address of the receiver.
/// @param _amount      Amount of local tokens to deposit.
/// @param _minGasLimit Minimum amount of gas that the bridge can be relayed with.
/// @param _extraData   Extra data to be sent with the transaction. Note that the recipient will
///                     not be triggered with this data, but it will be emitted and can be used
///                     to identify the transaction.
pub fn bridge_sol_to_handler(
    ctx: Context<BridgeSolTo>,
    remote_token: [u8; 20],
    to: [u8; 20],
    amount: u64,
    min_gas_limit: u32,
    extra_data: Vec<u8>,
) -> Result<()> {
    let program_id: &[u8] = ctx.program_id.as_ref();
    initiate_bridge_sol(
        program_id,
        &ctx.accounts.system_program,
        &ctx.accounts.user.to_account_info(),
        &ctx.accounts.vault.to_account_info(),
        &mut ctx.accounts.msg_state,
        ctx.accounts.user.key(),
        remote_token,
        to,
        amount,
        min_gas_limit,
        extra_data,
    )
}

/// @notice Sends SPL tokens or SOL to a receiver's address on Base.
///
/// @param _remoteToken Address of the corresponding token on Base.
/// @param _to          Address of the receiver.
/// @param _amount      Amount of local tokens to deposit.
/// @param _minGasLimit Minimum amount of gas that the bridge can be relayed with.
/// @param _extraData   Extra data to be sent with the transaction. Note that the recipient will
///                     not be triggered with this data, but it will be emitted and can be used
///                     to identify the transaction.
pub fn bridge_tokens_to_handler(
    ctx: Context<BridgeTokensTo>,
    remote_token: [u8; 20],
    to: [u8; 20],
    amount: u64,
    min_gas_limit: u32,
    extra_data: Vec<u8>,
) -> Result<()> {
    let program_id: &[u8] = ctx.program_id.as_ref();
    initiate_bridge_tokens(
        program_id,
        &ctx.accounts.token_program,
        &ctx.accounts.user.to_account_info(),
        &ctx.accounts.from_token_account,
        &ctx.accounts.vault_token_account,
        &mut ctx.accounts.msg_state,
        ctx.accounts.user.key(),
        ctx.accounts.mint.key(),
        remote_token,
        to,
        amount,
        min_gas_limit,
        extra_data,
    )
}

fn initiate_bridge_sol<'info>(
    program_id: &[u8],
    system_program: &Program<'info, System>,
    user: &AccountInfo<'info>,
    vault: &AccountInfo<'info>,
    msg_state: &mut Account<'info, Messenger>,
    from: Pubkey,
    remote_token: [u8; 20],
    to: [u8; 20],
    amount: u64,
    min_gas_limit: u32,
    extra_data: Vec<u8>,
) -> Result<()> {
    // Transfer `amount` of local_token from user to vault
    // Transfer lamports from user to vault PDA
    let cpi_context = CpiContext::new(
        system_program.to_account_info(),
        anchor_lang::system_program::Transfer {
            from: user.clone(),
            to: vault.clone(),
        },
    );
    anchor_lang::system_program::transfer(cpi_context, amount)?;

    emit_event_and_send_message(
        program_id,
        user,
        msg_state,
        from,
        NATIVE_SOL_PUBKEY,
        remote_token,
        to,
        amount,
        min_gas_limit,
        extra_data,
    )
}

fn initiate_bridge_tokens<'info>(
    program_id: &[u8],
    token_program: &Program<'info, Token>,
    user_account_info: &AccountInfo<'info>,
    user_spl_token_account: &Account<'info, TokenAccount>,
    vault_spl_token_account: &Account<'info, TokenAccount>,
    msg_state: &mut Account<'info, Messenger>,
    sender_on_solana_pubkey: Pubkey,
    token_on_solana_mint_pubkey: Pubkey,
    token_on_base_address: [u8; 20],
    receiver_on_base_address: [u8; 20],
    amount_to_bridge: u64,
    min_gas_limit_for_relay: u32,
    extra_data_bytes: Vec<u8>,
) -> Result<()> {
    if token_on_solana_mint_pubkey == NATIVE_SOL_PUBKEY {
        return err!(BridgeError::InvalidSolUsage);
    }

    // SPL Token Transfer
    let cpi_accounts = anchor_spl::token::Transfer {
        from: user_spl_token_account.to_account_info(),
        to: vault_spl_token_account.to_account_info(),
        authority: user_account_info.clone(),
    };
    let cpi_program = token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    anchor_spl::token::transfer(cpi_ctx, amount_to_bridge)?;

    emit_event_and_send_message(
        program_id,
        user_account_info,
        msg_state,
        sender_on_solana_pubkey,
        token_on_solana_mint_pubkey,
        token_on_base_address,
        receiver_on_base_address,
        amount_to_bridge,
        min_gas_limit_for_relay,
        extra_data_bytes,
    )
}

fn emit_event_and_send_message<'info>(
    program_id: &[u8],
    user: &AccountInfo<'info>,
    msg_state: &mut Account<'info, Messenger>,
    from: Pubkey,
    local_token: Pubkey,
    remote_token: [u8; 20],
    to: [u8; 20],
    amount: u64,
    min_gas_limit: u32,
    extra_data: Vec<u8>,
) -> Result<()> {
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

    // Add to (32 bytes) - pad 20-byte address to 32 bytes
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
    #[msg("Cannot bridge SOL here")]
    InvalidSolUsage,
}
