use anchor_lang::prelude::*;
use anchor_lang::solana_program::keccak;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};

use crate::{
    BridgePayload, Deposit, Message, Messenger, ASSOCIATED_TOKEN_PROGRAM_ID, DEPOSIT_SEED,
    FINALIZE_BRIDGE_TOKEN_SELECTOR, MESSENGER_SEED, NATIVE_SOL_PUBKEY, OTHER_BRIDGE,
    TOKEN_PROGRAM_ID, VAULT_SEED, VERSION,
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
pub struct BridgeTokensTo<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK: This is the vault PDA. For SOL, it receives SOL. For SPL, it's the authority for vault_token_account.
    #[account(
        mut,
        seeds = [VAULT_SEED, VERSION.to_le_bytes().as_ref()],
        bump
    )]
    pub vault: AccountInfo<'info>,

    #[account(mut, seeds = [MESSENGER_SEED, VERSION.to_le_bytes().as_ref()], bump)]
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
        seeds = [VAULT_SEED, VERSION.to_le_bytes().as_ref()],
        bump
    )]
    pub vault: AccountInfo<'info>,

    #[account(mut, seeds = [MESSENGER_SEED, VERSION.to_le_bytes().as_ref()], bump)]
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
    let bridge_params = BridgeCallParams {
        remote_token,
        to,
        amount,
        min_gas_limit,
        extra_data,
    };
    initiate_bridge_sol(
        &ctx.accounts.system_program,
        &ctx.accounts.user.to_account_info(),
        &ctx.accounts.vault.to_account_info(),
        &mut ctx.accounts.msg_state,
        &mut ctx.accounts.deposit,
        ctx.accounts.user.key(),
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
    let bridge_params = BridgeCallParams {
        remote_token,
        to,
        amount,
        min_gas_limit,
        extra_data,
    };
    initiate_bridge_tokens(
        &ctx.accounts.token_program,
        &ctx.accounts.user, // Pass the Signer directly
        &ctx.accounts.from_token_account,
        &ctx.accounts.vault_token_account,
        &mut ctx.accounts.msg_state,
        &mut ctx.accounts.deposit,
        ctx.accounts.user.key(),
        ctx.accounts.mint.key(),
        &bridge_params,
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
    vault_pda_info: &AccountInfo<'info>,
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
        let recipient_info = find_account_info_by_key(
            account_infos,
            &payload.to,
            ProgramError::NotEnoughAccountKeys,
        )?;

        let (deposit_acct_key, _) = Pubkey::find_program_address(
            &[
                DEPOSIT_SEED,
                payload.local_token.as_ref(),
                payload.remote_token.as_ref(),
            ],
            &crate::ID,
        );
        let deposit_info = find_account_info_by_key(
            account_infos,
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

        if payload.local_token == NATIVE_SOL_PUBKEY {
            **vault_pda_info.try_borrow_mut_lamports()? -= payload.amount;
            **recipient_info.try_borrow_mut_lamports()? += payload.amount;
        } else {
            let local_token_mint_info = find_account_info_by_key(
                account_infos,
                &payload.local_token,
                ProgramError::NotEnoughAccountKeys,
            )?;

            let (vault_ata_key, _) = Pubkey::find_program_address(
                &[
                    vault_pda_info.key.to_bytes().as_ref(),
                    TOKEN_PROGRAM_ID.to_bytes().as_ref(),
                    local_token_mint_info.key.to_bytes().as_ref(),
                ],
                &ASSOCIATED_TOKEN_PROGRAM_ID,
            );

            let vault_ata_info = find_account_info_by_key(
                account_infos,
                &vault_ata_key,
                ProgramError::NotEnoughAccountKeys,
            )?;

            // Prepare seeds for the vault PDA
            let version_bytes = VERSION.to_le_bytes();
            let vault_pda_seeds: &[&[u8]] = &[VAULT_SEED, version_bytes.as_ref()];

            spl_transfer_pda_signed(
                local_token_mint_info,
                vault_ata_info,
                recipient_info,
                vault_pda_info,
                vault_pda_seeds, // Pass the combined seeds
                &crate::ID,      // Program ID that owns the vault PDA
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
    user_account_info: &AccountInfo<'info>,
    vault_account_info: &AccountInfo<'info>,
    msg_state: &mut Account<'info, Messenger>,
    deposit: &mut Account<'info, Deposit>,
    from_solana_pubkey: Pubkey,
    bridge_params: &BridgeCallParams,
) -> Result<()> {
    // Transfer `amount` of local_token from user to vault
    // Transfer lamports from user to vault PDA
    let cpi_context = CpiContext::new(
        system_program.to_account_info(),
        anchor_lang::system_program::Transfer {
            from: user_account_info.clone(),
            to: vault_account_info.clone(),
        },
    );
    anchor_lang::system_program::transfer(cpi_context, bridge_params.amount)?;

    emit_event_and_send_message(
        msg_state,
        deposit,
        from_solana_pubkey,
        NATIVE_SOL_PUBKEY, // local_token is SOL
        bridge_params,
    )
}

fn initiate_bridge_tokens<'info>(
    token_program: &Program<'info, Token>,
    user_signer: &Signer<'info>, // Changed from user_account_info
    user_spl_token_account: &Account<'info, TokenAccount>,
    vault_spl_token_account: &Account<'info, TokenAccount>,
    msg_state: &mut Account<'info, Messenger>,
    deposit: &mut Account<'info, Deposit>,
    sender_on_solana_pubkey: Pubkey,
    token_on_solana_mint_pubkey: Pubkey,
    bridge_params: &BridgeCallParams,
) -> Result<()> {
    if token_on_solana_mint_pubkey == NATIVE_SOL_PUBKEY {
        return err!(BridgeError::InvalidSolUsage);
    }

    spl_transfer_user_signed(
        &token_program.to_account_info(),
        user_spl_token_account,
        vault_spl_token_account,
        user_signer,
        bridge_params.amount,
    )?;

    emit_event_and_send_message(
        msg_state,
        deposit,
        sender_on_solana_pubkey,
        token_on_solana_mint_pubkey,
        bridge_params,
    )
}

fn emit_event_and_send_message<'info>(
    msg_state: &mut Account<'info, Messenger>,
    deposit: &mut Account<'info, Deposit>,
    from_solana_pubkey: Pubkey,
    local_token_mint_pubkey: Pubkey,
    bridge_params: &BridgeCallParams,
) -> Result<()> {
    deposit.balance += bridge_params.amount;

    emit!(TokenBridgeInitiated {
        local_token: local_token_mint_pubkey,
        remote_token: bridge_params.remote_token,
        from: from_solana_pubkey,
        to: bridge_params.to,
        amount: bridge_params.amount,
        extra_data: bridge_params.extra_data.clone()
    });

    messenger::send_message_internal(
        msg_state,
        local_bridge_pubkey(),
        OTHER_BRIDGE,
        encode_bridge_payload_for_base(
            // Renamed for clarity
            bridge_params.remote_token,
            local_token_mint_pubkey,
            from_solana_pubkey,
            bridge_params.to,
            bridge_params.amount,
            &bridge_params.extra_data,
        ),
        bridge_params.min_gas_limit,
    )
}

/// Encodes the payload for the `Base.StandardBridge.finalizeBridgeToken` function.
fn encode_bridge_payload_for_base(
    remote_token_on_base: [u8; 20], // Address of the ERC20 on Base
    local_token_on_solana: Pubkey,  // Address of the token on this chain
    from_on_solana: Pubkey,         // Address of the sender on Solana
    to_on_base: [u8; 20],           // Address of the receiver on Base
    amount: u64,
    extra_data: &Vec<u8>,
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

// TODO: need to implement
fn is_owned_by_bridge(_token: Pubkey) -> bool {
    return false;
}

/// Transfers SPL tokens when the authority is a user Signer.
fn spl_transfer_user_signed<'info>(
    token_program_info: &AccountInfo<'info>,
    from_token_account: &Account<'info, TokenAccount>,
    to_token_account: &Account<'info, TokenAccount>,
    authority_signer: &Signer<'info>,
    amount: u64,
) -> Result<()> {
    let cpi_accounts = anchor_spl::token::Transfer {
        from: from_token_account.to_account_info(),
        to: to_token_account.to_account_info(),
        authority: authority_signer.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(token_program_info.clone(), cpi_accounts);
    anchor_spl::token::transfer(cpi_ctx, amount)?;
    Ok(())
}

/// Transfers SPL tokens when the authority is a PDA.
fn spl_transfer_pda_signed<'info>(
    token_program_info: &AccountInfo<'info>,
    from_token_account_info: &AccountInfo<'info>,
    to_token_account_info: &AccountInfo<'info>,
    authority_pda_info: &AccountInfo<'info>, // The PDA account that is the authority
    pda_base_seeds: &[&[u8]], // Changed: Now accepts a slice of byte slices for seeds
    pda_owning_program_id: &Pubkey, // e.g., &crate::ID
    amount: u64,
) -> Result<()> {
    let (_pda_key, bump_value) =
        Pubkey::find_program_address(pda_base_seeds, pda_owning_program_id);

    let bump_array = [bump_value]; // Creates [u8; 1]

    // Construct the full seeds array for signing: base_seeds + bump_array as a slice
    let mut actual_signer_seeds_elements: Vec<&[u8]> = Vec::with_capacity(pda_base_seeds.len() + 1);
    actual_signer_seeds_elements.extend_from_slice(pda_base_seeds);
    actual_signer_seeds_elements.push(&bump_array); // Add the bump seed slice `&[u8]`

    let cpi_signer_seeds_outer: &[&[&[u8]]] = &[&actual_signer_seeds_elements];

    let cpi_accounts = anchor_spl::token::Transfer {
        from: from_token_account_info.clone(),
        to: to_token_account_info.clone(),
        authority: authority_pda_info.clone(),
    };
    let cpi_ctx = CpiContext::new(token_program_info.clone(), cpi_accounts)
        .with_signer(cpi_signer_seeds_outer);
    anchor_spl::token::transfer(cpi_ctx, amount)?;
    Ok(())
}

// Helper to find an AccountInfo by its key from a slice of AccountInfos.
fn find_account_info_by_key<'a, 'info>(
    account_infos: &'a [AccountInfo<'info>],
    key: &Pubkey,
    error_if_not_found: ProgramError,
) -> Result<&'a AccountInfo<'info>> {
    account_infos
        .iter()
        .find(|acc_info| acc_info.key == key)
        .ok_or(error_if_not_found.into())
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
