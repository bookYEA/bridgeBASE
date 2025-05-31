use anchor_lang::prelude::*;
use anchor_lang::solana_program::keccak;
use anchor_lang::solana_program::program_option::COption;
use anchor_spl::{
    token::{Burn, Mint, Token, TokenAccount},
    token_interface::{self, MintTo},
};

use crate::{
    BridgePayload, Deposit, Message, Messenger, AUTHORITY_VAULT_SEED, BRIDGE_SEED, DEPOSIT_SEED,
    FINALIZE_BRIDGE_TOKEN_SELECTOR, GAS_FEE_RECEIVER, MESSENGER_SEED, MINT_SEED, NATIVE_SOL_PUBKEY,
    REMOTE_BRIDGE, TOKEN_PROGRAM_ID, TOKEN_VAULT_SEED, VERSION,
};

use super::{messenger, MessengerError};

// Constants for ABI encoding
pub const ABI_ADDRESS_PARAM_SIZE: usize = 32;
pub const ABI_U64_PARAM_SIZE: usize = 32;
pub const ABI_DYNAMIC_OFFSET_SIZE: usize = 32;

// Number of static 32-byte parameters before the dynamic `extraData` in finalizeBridgeToken
pub const ABI_FINALIZE_BRIDGE_STATIC_PARAMS_COUNT: usize = 6;
pub const ABI_FINALIZE_BRIDGE_STATIC_PART_SIZE: usize =
    ABI_FINALIZE_BRIDGE_STATIC_PARAMS_COUNT * ABI_ADDRESS_PARAM_SIZE;

/// Parameters for initiating a token bridge.
struct BridgeCallParams {
    remote_token: [u8; 20],
    to: [u8; 20],
    amount: u64,
    min_gas_limit: u32,
    extra_data: Vec<u8>,
}

#[derive(Accounts)]
#[instruction(remote_token: [u8; 20])]
pub struct BridgeSolTo<'info> {
    // Portal accounts
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut, address = GAS_FEE_RECEIVER)]
    /// CHECK: This is the hardcoded gas fee receiver account.
    pub gas_fee_receiver: AccountInfo<'info>,

    pub system_program: Program<'info, System>,

    // Messenger accounts
    #[account(mut, seeds = [MESSENGER_SEED, VERSION.to_le_bytes().as_ref()], bump)]
    pub messenger: Account<'info, Messenger>,

    // Bridge accounts
    /// CHECK: This is the vault authority PDA.
    ///        - For SOL, it receives SOL.
    ///        - For SPL, it's the authority of the vault token account.
    #[account(
        mut,
        seeds = [AUTHORITY_VAULT_SEED, VERSION.to_le_bytes().as_ref()],
        bump
    )]
    pub authority_vault: AccountInfo<'info>,

    #[account(
        init_if_needed,
        payer = user,
        space = 8 + Deposit::INIT_SPACE,
        seeds = [DEPOSIT_SEED, NATIVE_SOL_PUBKEY.as_ref(), remote_token.as_ref()],
        bump
    )]
    pub deposit: Account<'info, Deposit>,
}

#[derive(Accounts)]
#[instruction(remote_token: [u8; 20])]
pub struct BridgeTokensTo<'info> {
    // Portal accounts
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut, address = GAS_FEE_RECEIVER)]
    /// CHECK: This is the hardcoded gas fee receiver account.
    pub gas_fee_receiver: AccountInfo<'info>,

    pub system_program: Program<'info, System>,

    // Messenger accounts
    #[account(mut, seeds = [MESSENGER_SEED, VERSION.to_le_bytes().as_ref()], bump)]
    pub messenger: Account<'info, Messenger>,

    // Bridge accounts
    /// CHECK: This is the vault authority PDA.
    ///        - For SOL, it receives SOL.
    ///        - For SPL, it's the authority of the vault token account.
    #[account(
        mut,
        seeds = [AUTHORITY_VAULT_SEED, VERSION.to_le_bytes().as_ref()],
        bump
    )]
    pub authority_vault: AccountInfo<'info>,

    // SPL Token specific accounts.
    // These accounts must be provided by the client.
    #[account(mut)]
    pub mint: Account<'info, Mint>,

    #[account(mut)]
    pub from_token_account: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        seeds = [TOKEN_VAULT_SEED, mint.key().as_ref(), VERSION.to_le_bytes().as_ref()],
        bump,
        payer = user,
        token::mint = mint,
        token::authority = authority_vault
    )]
    pub token_vault: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = user,
        space = 8 + Deposit::INIT_SPACE,
        seeds = [DEPOSIT_SEED, mint.key().as_ref(), remote_token.as_ref()],
        bump
    )]
    pub deposit: Account<'info, Deposit>,

    pub token_program: Program<'info, Token>,
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
    let bridge_params = BridgeCallParams {
        remote_token,
        to,
        amount,
        min_gas_limit,
        extra_data,
    };

    // Transfer lamports from user to vault PDA
    let cpi_context = CpiContext::new(
        ctx.accounts.system_program.to_account_info(),
        anchor_lang::system_program::Transfer {
            from: ctx.accounts.user.to_account_info(),
            to: ctx.accounts.authority_vault.to_account_info(),
        },
    );
    anchor_lang::system_program::transfer(cpi_context, bridge_params.amount)?;

    emit_event_and_send_message(
        &ctx.accounts.system_program,
        &ctx.accounts.gas_fee_receiver,
        &ctx.accounts.user,
        &mut ctx.accounts.messenger,
        &mut ctx.accounts.deposit,
        NATIVE_SOL_PUBKEY,
        &bridge_params,
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
    require!(
        ctx.accounts.mint.key() != NATIVE_SOL_PUBKEY,
        BridgeError::InvalidSolUsage
    );

    let bridge_params = BridgeCallParams {
        remote_token,
        to,
        amount,
        min_gas_limit,
        extra_data,
    };

    if is_owned_by_bridge(&ctx.accounts.mint.to_account_info())? {
        let cpi_context = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Burn {
                mint: ctx.accounts.mint.to_account_info(),
                from: ctx.accounts.from_token_account.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );
        anchor_spl::token::burn(cpi_context, bridge_params.amount)?;
    } else {
        let cpi_accounts = anchor_spl::token::Transfer {
            from: ctx.accounts.from_token_account.to_account_info(),
            to: ctx.accounts.token_vault.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        anchor_spl::token::transfer(cpi_ctx, bridge_params.amount)?;
    }

    emit_event_and_send_message(
        &ctx.accounts.system_program,
        &ctx.accounts.gas_fee_receiver,
        &ctx.accounts.user,
        &mut ctx.accounts.messenger,
        &mut ctx.accounts.deposit,
        ctx.accounts.mint.key(),
        &bridge_params,
    )
}

/// @notice Finalizes a Token bridge on this chain. Can only be triggered by the remote
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
    message: &mut Account<'info, Message>,
    authority_vault: &AccountInfo<'info>,
    remaining_accounts: &'info [AccountInfo<'info>],
    payload: BridgePayload,
) -> Result<()> {
    require!(
        message.messenger_caller == REMOTE_BRIDGE,
        BridgeError::OnlyRemoteBridgeCanCall
    );

    // On L1 this function will check the Portal for its paused status.
    // On L2 this function should be a no-op, because paused will always return false.
    require!(!messenger::paused(), MessengerError::BridgeIsPaused);

    let recipient_info = find_account_info_by_key(
        remaining_accounts,
        &payload.to,
        ProgramError::NotEnoughAccountKeys,
    )?;

    if payload.local_token == NATIVE_SOL_PUBKEY {
        decrement_vault_balance(remaining_accounts, &payload)?;
        **authority_vault.try_borrow_mut_lamports()? -= payload.amount;
        **recipient_info.try_borrow_mut_lamports()? += payload.amount;
    } else {
        let mint_info = find_account_info_by_key(
            remaining_accounts,
            &payload.local_token,
            ProgramError::NotEnoughAccountKeys,
        )?;

        if is_owned_by_bridge(mint_info)? {
            let token_program_info = find_account_info_by_key(
                remaining_accounts,
                &TOKEN_PROGRAM_ID,
                ProgramError::NotEnoughAccountKeys,
            )?;

            let mint_account = Account::<Mint>::try_from(mint_info)?;
            let decimals_bytes = mint_account.decimals.to_le_bytes(); // TODO: Fix this when we correctly implement decimals clamping.
            let seeds: &[&[u8]] = &[
                MINT_SEED,
                payload.remote_token.as_ref(),
                decimals_bytes.as_ref(),
            ];

            let (mint_key, bump_value) = Pubkey::find_program_address(seeds, &crate::ID);
            require_keys_eq!(mint_key, mint_info.key(), BridgeError::InvalidTokenPair);
            let bump_slice = [bump_value];

            let mut seeds_and_bump: Vec<&[u8]> = Vec::with_capacity(seeds.len() + 1);
            seeds_and_bump.extend_from_slice(seeds);
            seeds_and_bump.push(&bump_slice);
            let seeds_and_bump: &[&[&[u8]]] = &[&seeds_and_bump];

            let cpi_context = CpiContext::new(
                token_program_info.clone(),
                MintTo {
                    mint: mint_info.clone(),
                    to: recipient_info.clone(),
                    authority: mint_info.clone(),
                },
            )
            .with_signer(seeds_and_bump);
            token_interface::mint_to(cpi_context, payload.amount)?;
        } else {
            decrement_vault_balance(remaining_accounts, &payload)?;

            unlock_from_vault(
                authority_vault,
                mint_info,
                recipient_info,
                remaining_accounts,
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

fn decrement_vault_balance<'info>(
    remaining_accounts: &'info [AccountInfo<'info>],
    payload: &BridgePayload,
) -> Result<()> {
    let (deposit_acct_key, _) = Pubkey::find_program_address(
        &[
            DEPOSIT_SEED,
            payload.local_token.as_ref(),
            payload.remote_token.as_ref(),
        ],
        &crate::ID,
    );

    let deposit_info = find_account_info_by_key(
        remaining_accounts,
        &deposit_acct_key,
        ProgramError::NotEnoughAccountKeys, // Example error
    )?;

    let mut deposit = Account::<Deposit>::try_from(deposit_info)?;
    if deposit.balance < payload.amount {
        msg!(
            "Insufficient balance. Has {:?} needs {:?}",
            deposit.balance,
            payload.amount
        );
        return err!(BridgeError::InsufficientBalance);
    }

    deposit.balance -= payload.amount;

    Ok(())
}

fn emit_event_and_send_message<'info>(
    system_program: &Program<'info, System>,
    gas_fee_receiver: &AccountInfo<'info>,
    user: &Signer<'info>,
    messenger: &mut Account<'info, Messenger>,
    deposit: &mut Account<'info, Deposit>,
    local_token_mint_pubkey: Pubkey,
    bridge_params: &BridgeCallParams,
) -> Result<()> {
    deposit.balance += bridge_params.amount;

    emit!(TokenBridgeInitiated {
        local_token: local_token_mint_pubkey,
        remote_token: bridge_params.remote_token,
        from: user.key(),
        to: bridge_params.to,
        amount: bridge_params.amount,
        extra_data: bridge_params.extra_data.clone()
    });

    messenger::send_message_internal(
        system_program,
        user,
        gas_fee_receiver,
        messenger,
        local_bridge_pubkey(),
        REMOTE_BRIDGE,
        encode_finalize_brigde_token_call(
            bridge_params.remote_token,
            local_token_mint_pubkey,
            user.key(),
            bridge_params.to,
            bridge_params.amount,
            &bridge_params.extra_data,
        ),
        bridge_params.min_gas_limit,
    )
}

/// Encodes the payload for the `Base.StandardBridge.finalizeBridgeToken` function.
fn encode_finalize_brigde_token_call(
    remote_token_on_base: [u8; 20], // Address of the ERC20 on Base
    local_token_on_solana: Pubkey,  // Address of the token on this chain
    from_on_solana: Pubkey,         // Address of the sender on Solana
    to_on_base: [u8; 20],           // Address of the receiver on Base
    amount: u64,
    extra_data: &[u8],
) -> Vec<u8> {
    let mut encoded_payload = Vec::new();

    // Selector for Base.StandardBridge.finalizeBridgeToken: 0x2d916920 (4 bytes)
    encoded_payload.extend_from_slice(&FINALIZE_BRIDGE_TOKEN_SELECTOR);

    // Parameter 1: remoteToken (address)
    append_padded_abi_bytes(
        &mut encoded_payload,
        &remote_token_on_base,
        ABI_ADDRESS_PARAM_SIZE,
    );

    // Parameter 2: localToken (address) - Pubkey is 32 bytes
    append_padded_abi_bytes(
        &mut encoded_payload,
        local_token_on_solana.as_ref(),
        ABI_ADDRESS_PARAM_SIZE,
    );

    // Parameter 3: _from (address) - Pubkey is 32 bytes
    append_padded_abi_bytes(
        &mut encoded_payload,
        from_on_solana.as_ref(),
        ABI_ADDRESS_PARAM_SIZE,
    );

    // Parameter 4: _to (address)
    append_padded_abi_bytes(&mut encoded_payload, &to_on_base, ABI_ADDRESS_PARAM_SIZE);

    // Parameter 5: _amount (uint256)
    append_padded_abi_bytes(
        &mut encoded_payload,
        &amount.to_be_bytes(),
        ABI_U64_PARAM_SIZE,
    );

    // Parameter 6: _extraData (bytes) - This is a dynamic type, so we add an offset first.
    // Offset points to the start of the extraData's length, which is after all static params.
    append_padded_abi_bytes(
        &mut encoded_payload,
        &(ABI_FINALIZE_BRIDGE_STATIC_PART_SIZE as u64).to_be_bytes(),
        ABI_DYNAMIC_OFFSET_SIZE,
    );

    // Dynamic part: extraData
    // Length of extraData
    append_padded_abi_bytes(
        &mut encoded_payload,
        &(extra_data.len() as u64).to_be_bytes(),
        ABI_U64_PARAM_SIZE,
    );

    // Actual extraData
    encoded_payload.extend_from_slice(extra_data);

    // Pad extraData to a multiple of 32 bytes
    let padding_len = (ABI_ADDRESS_PARAM_SIZE - (extra_data.len() % ABI_ADDRESS_PARAM_SIZE))
        % ABI_ADDRESS_PARAM_SIZE;
    encoded_payload.extend_from_slice(&vec![0u8; padding_len]);

    encoded_payload
}

// Helper function to pad data (e.g. addresses, uints) to a specific ABI length (typically 32 bytes)
// and append it to the main byte vector. Data is right-aligned (big-endian).
fn append_padded_abi_bytes(encoded_vec: &mut Vec<u8>, data_slice: &[u8], abi_length: usize) {
    let mut padded_data = vec![0u8; abi_length];
    let data_len = data_slice.len();
    if data_len > abi_length {
        // This case should ideally be handled before calling, e.g. by slicing `data_slice`
        // For now, we'll truncate if it's an address like [u8;20] being put into 32 bytes (left padding)
        // or take the last bytes if it's a u64 into 32 bytes.
        // Standard behavior for ABI encoding is to pad smaller values, and for larger values, it depends.
        // For numbers, usually takes lower-order bytes. For byte arrays, it's often an error or specific truncation.
        // Here, we assume data_slice.len() <= abi_length, and we are padding.
        // For addresses like [u8; 20] into 32 bytes, they are typically padded at the start.
        // For numbers like u64 (8 bytes) into 32 bytes, they are also padded at the start.
        padded_data[(abi_length - data_len)..].copy_from_slice(data_slice);
    } else {
        // Pad at the beginning (right-align)
        padded_data[(abi_length - data_len)..].copy_from_slice(data_slice);
    }
    encoded_vec.extend_from_slice(&padded_data);
}

fn is_owned_by_bridge(mint_info: &AccountInfo<'_>) -> Result<bool> {
    // Ensure the account is owned by the SPL Token program
    if *mint_info.owner != TOKEN_PROGRAM_ID {
        // Not an SPL Mint account or owned by the wrong token program.
        // Returning false as it's not "owned by bridge" in the intended way.
        return Ok(false);
    }

    // Attempt to deserialize the mint data. This will propagate an error if deserialization fails (e.g. wrong data, length).
    let mint = anchor_spl::token::Mint::try_deserialize(&mut &mint_info.try_borrow_data()?[..])?;

    // Check if the mint is initialized and its mint authority is the mint PDA.
    Ok(mint.is_initialized && mint.mint_authority == COption::Some(mint_info.key()))
}

/// Transfers SPL tokens from a vault to a recipient.
fn unlock_from_vault<'info>(
    authority_vault: &AccountInfo<'info>,
    mint_info: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    remaining_accounts: &'info [AccountInfo<'info>],
    amount: u64,
) -> Result<()> {
    let token_program_info = find_account_info_by_key(
        remaining_accounts,
        &TOKEN_PROGRAM_ID,
        ProgramError::NotEnoughAccountKeys,
    )?;

    let (token_vault_key, _) = Pubkey::find_program_address(
        &[
            TOKEN_VAULT_SEED,
            mint_info.key.to_bytes().as_ref(),
            VERSION.to_le_bytes().as_ref(),
        ],
        &crate::ID,
    );

    let token_vault_info = find_account_info_by_key(
        remaining_accounts,
        &token_vault_key,
        ProgramError::NotEnoughAccountKeys,
    )?;

    let version_bytes = VERSION.to_le_bytes();
    let seeds: &[&[u8]] = &[AUTHORITY_VAULT_SEED, version_bytes.as_ref()];
    let (_, bump_value) = Pubkey::find_program_address(seeds, &crate::ID);
    let bump_array = [bump_value];

    let mut seeds_and_bump: Vec<&[u8]> = Vec::with_capacity(seeds.len() + 1);
    seeds_and_bump.extend_from_slice(seeds);
    seeds_and_bump.push(&bump_array);
    let seeds_and_bump: &[&[&[u8]]] = &[&seeds_and_bump];

    let cpi_ctx = CpiContext::new(
        token_program_info.clone(),
        anchor_spl::token::Transfer {
            from: token_vault_info.clone(),
            to: to.clone(),
            authority: authority_vault.clone(),
        },
    )
    .with_signer(seeds_and_bump);

    anchor_spl::token::transfer(cpi_ctx, amount)?;
    Ok(())
}

// TODO: Instead of searching for accounts, we might want to force them to be at a given index.
// Helper to find an AccountInfo by its key from a slice of AccountInfos.
fn find_account_info_by_key<'a, 'info>(
    remaining_accounts: &'a [AccountInfo<'info>],
    key: &Pubkey,
    error_if_not_found: ProgramError,
) -> Result<&'a AccountInfo<'info>> {
    remaining_accounts
        .iter()
        .find(|acc_info| acc_info.key == key)
        .ok_or(error_if_not_found.into())
}

pub fn local_bridge_pubkey() -> Pubkey {
    // Equivalent to keccak256(abi.encodePacked(programId, "bridge"));
    let mut data_to_hash = Vec::new();
    data_to_hash.extend_from_slice(crate::ID.as_ref());
    data_to_hash.extend_from_slice(BRIDGE_SEED);
    let hash = keccak::hash(&data_to_hash);
    Pubkey::new_from_array(hash.to_bytes())
}

#[error_code]
pub enum BridgeError {
    #[msg("Cannot bridge SOL here")]
    InvalidSolUsage,
    #[msg("Only remote bridge can call")]
    OnlyRemoteBridgeCanCall,
    #[msg("Insufficient balance")]
    InsufficientBalance,
    #[msg("Invalid token pair")]
    InvalidTokenPair,
}
