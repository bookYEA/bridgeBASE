use anchor_lang::prelude::*;
use anchor_lang::solana_program::keccak;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};
use hex_literal::hex;

use crate::{
    messenger, BridgePayload, Deposit, Message, ASSOCIATED_TOKEN_PROGRAM_ID, DEPOSIT_SEED,
    MESSENGER_SEED, NATIVE_SOL_PUBKEY, OTHER_BRIDGE, TOKEN_PROGRAM_ID, VAULT_SEED,
};

use super::{Messenger, MessengerError};

#[derive(Accounts)]
#[instruction(remote_token: [u8; 20])]
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

    #[account(
        init_if_needed,
        payer = user,
        space = 8 + Deposit::INIT_SPACE,
        seeds = [DEPOSIT_SEED, mint.key().as_ref(), remote_token.as_ref()],
        bump
    )]
    pub deposit: Account<'info, Deposit>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
#[instruction(remote_token: [u8; 20])]
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

    #[account(
        init_if_needed,
        payer = user,
        space = 8 + Deposit::INIT_SPACE,
        seeds = [DEPOSIT_SEED, NATIVE_SOL_PUBKEY.as_ref(), remote_token.as_ref()],
        bump
    )]
    pub deposit: Account<'info, Deposit>,

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

#[event]
// Emitted when an ERC20 bridge is finalized on this chain.
pub struct TokenBridgeFinalized {
    pub local_token: Pubkey, // Address of the token on this chain. Default pubkey signifies SOL.
    pub remote_token: [u8; 20], // Address of the ERC20 on Base.
    pub from: [u8; 20],      // Address of the sender.
    pub to: Pubkey,          // Address of the receiver.
    pub amount: u64,         // Amount of tokens sent.
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
    initiate_bridge_sol(
        &ctx.accounts.system_program,
        &ctx.accounts.user.to_account_info(),
        &ctx.accounts.vault.to_account_info(),
        &mut ctx.accounts.msg_state,
        &mut ctx.accounts.deposit,
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
    initiate_bridge_tokens(
        &ctx.accounts.token_program,
        &ctx.accounts.user.to_account_info(),
        &ctx.accounts.from_token_account,
        &ctx.accounts.vault_token_account,
        &mut ctx.accounts.msg_state,
        &mut ctx.accounts.deposit,
        ctx.accounts.user.key(),
        ctx.accounts.mint.key(),
        remote_token,
        to,
        amount,
        min_gas_limit,
        extra_data,
    )
}

/// @notice Finalizes a Token bridge on this chain. Can only be triggered by the other
///         StandardBridge contract on Base.
/// @param _localToken  Address of the SPL token or native SOL on this chain.
/// @param _remoteToken Address of the corresponding token on Base.
/// @param _from        Address of the sender.
/// @param _to          Address of the receiver.
/// @param _amount      Amount of the token being bridged.
/// @param _extraData   Extra data to be sent with the transaction. Note that the recipient will
///                     not be triggered with this data, but it will be emitted and can be used
///                     to identify the transaction.
pub fn finalize_bridge_tokens<'info>(
    message_account: &mut Account<'info, Message>,
    vault: &AccountInfo<'info>,
    account_infos: &'info [AccountInfo<'info>],
    payload: BridgePayload,
) -> Result<()> {
    if message_account.sender != OTHER_BRIDGE {
        return err!(BridgeError::OnlyOtherBridgeCanCall);
    }

    // On L1 this function will check the Portal for its paused status.
    // On L2 this function should be a no-op, because paused will always return false.
    if messenger::paused() {
        return err!(MessengerError::BridgeIsPaused);
    }

    if is_owned_by_bridge(payload.local_token) {
        // TODO: implement
        // require(
        //     _isCorrectTokenPair(_localToken, _remoteToken),
        //     "StandardBridge: wrong remote token for Optimism Mintable ERC20 local token"
        // );

        // IOptimismMintableERC20(_localToken).mint(_to, _amount);
    } else {
        if payload.local_token == NATIVE_SOL_PUBKEY {
            return err!(BridgeError::InvalidSolUsage);
        } else {
            let (deposit_acct, _) = Pubkey::find_program_address(
                &[
                    DEPOSIT_SEED,
                    payload.local_token.as_ref(),
                    payload.remote_token.as_ref(),
                ],
                &crate::ID,
            );

            let deposit_info = account_infos
                .iter()
                .find(|x| x.key == &deposit_acct)
                .ok_or(ProgramError::InvalidArgument)?;

            let mut deposit = Account::<Deposit>::try_from(deposit_info)?;

            if deposit.balance < payload.amount {
                return err!(BridgeError::InsufficientBalance);
            }

            deposit.balance -= payload.amount;

            let token = account_infos
                .iter()
                .find(|x| x.key == &payload.local_token)
                .ok_or(ProgramError::InvalidArgument)?;

            let (vault_ata, _) = Pubkey::find_program_address(
                &[
                    vault.key.to_bytes().as_ref(),
                    TOKEN_PROGRAM_ID.to_bytes().as_ref(),
                    token.key.to_bytes().as_ref(),
                ],
                &ASSOCIATED_TOKEN_PROGRAM_ID,
            );

            let from = account_infos
                .iter()
                .find(|x| x.key == &vault_ata)
                .ok_or(ProgramError::InvalidArgument)?;
            let to = account_infos
                .iter()
                .find(|x| x.key == &payload.to)
                .ok_or(ProgramError::InvalidArgument)?;

            spl_transfer(
                token.clone(),
                from.clone(),
                to.clone(),
                vault.clone(),
                payload.amount,
            )?;
        }
    }

    emit!(TokenBridgeFinalized {
        local_token: payload.local_token,
        remote_token: payload.remote_token,
        from: payload.from,
        to: payload.to,
        amount: payload.amount,
        extra_data: payload.extra_data
    });

    Ok(())
}

fn initiate_bridge_sol<'info>(
    system_program: &Program<'info, System>,
    user: &AccountInfo<'info>,
    vault: &AccountInfo<'info>,
    msg_state: &mut Account<'info, Messenger>,
    deposit: &mut Account<'info, Deposit>,
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
        msg_state,
        deposit,
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
    token_program: &Program<'info, Token>,
    user_account_info: &AccountInfo<'info>,
    user_spl_token_account: &Account<'info, TokenAccount>,
    vault_spl_token_account: &Account<'info, TokenAccount>,
    msg_state: &mut Account<'info, Messenger>,
    deposit: &mut Account<'info, Deposit>,
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

    spl_transfer(
        token_program.to_account_info(),
        user_spl_token_account.to_account_info(),
        vault_spl_token_account.to_account_info(),
        user_account_info.clone(),
        amount_to_bridge,
    )?;

    emit_event_and_send_message(
        msg_state,
        deposit,
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
    msg_state: &mut Account<'info, Messenger>,
    deposit: &mut Account<'info, Deposit>,
    from: Pubkey,
    local_token: Pubkey,
    remote_token: [u8; 20],
    to: [u8; 20],
    amount: u64,
    min_gas_limit: u32,
    extra_data: Vec<u8>,
) -> Result<()> {
    deposit.balance += amount;

    emit!(TokenBridgeInitiated {
        local_token,
        remote_token,
        from,
        to,
        amount,
        extra_data: extra_data.clone()
    });

    messenger::send_message_internal(
        msg_state,
        local_bridge_pubkey(),
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

// TODO: need to implement
fn is_owned_by_bridge(_token: Pubkey) -> bool {
    return false;
}

fn spl_transfer<'info>(
    token: AccountInfo<'info>,
    from: AccountInfo<'info>,
    to: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    amount: u64,
) -> Result<()> {
    let cpi_accounts = anchor_spl::token::Transfer {
        from,
        to,
        authority,
    };
    let mut cpi_ctx = CpiContext::new(token, cpi_accounts);
    let (_, bump) = Pubkey::find_program_address(&[VAULT_SEED], &crate::ID);
    let binding: &[&[&[u8]]] = &[&[VAULT_SEED, &[bump]]];
    cpi_ctx.signer_seeds = binding;
    anchor_spl::token::transfer(cpi_ctx, amount)
}

pub fn local_bridge_pubkey() -> Pubkey {
    // Equivalent to keccak256(abi.encodePacked(programId, "bridge"));
    let mut data_to_hash = Vec::new();
    data_to_hash.extend_from_slice(crate::ID.as_ref());
    data_to_hash.extend_from_slice(b"bridge");
    let hash = keccak::hash(&data_to_hash);
    return Pubkey::new_from_array(hash.to_bytes());
}

#[error_code]
pub enum BridgeError {
    #[msg("Cannot bridge SOL here")]
    InvalidSolUsage,
    #[msg("Only other bridge can call")]
    OnlyOtherBridgeCanCall,
    #[msg("Insufficient balance")]
    InsufficientBalance,
}
