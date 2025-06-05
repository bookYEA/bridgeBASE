use anchor_lang::prelude::*;
use anchor_lang::solana_program::keccak;
use anchor_lang::solana_program::program_option::COption;
use anchor_spl::{
    token::{Burn, Mint, Token, TokenAccount},
    token_interface::{self, MintTo},
};

use crate::{
    BridgePayload, Message, Messenger, BRIDGE_SEED, FINALIZE_BRIDGE_TOKEN_SELECTOR,
    GAS_FEE_RECEIVER, MESSENGER_SEED, MINT_SEED, NATIVE_SOL_PUBKEY, REMOTE_BRIDGE, SOL_VAULT_SEED,
    TOKEN_PROGRAM_ID, TOKEN_VAULT_SEED,
};

use super::{messenger, MessengerError};

// Constants for ABI encoding
/// Size of an address parameter in ABI encoding
pub const ABI_ADDRESS_PARAM_SIZE: usize = 32;
/// Size of a uint64 parameter in ABI encoding
pub const ABI_U64_PARAM_SIZE: usize = 32;
/// Size of a dynamic offset in ABI encoding
pub const ABI_DYNAMIC_OFFSET_SIZE: usize = 32;

/// Number of static 32-byte parameters before the dynamic `extraData` in finalizeBridgeToken
pub const ABI_FINALIZE_BRIDGE_STATIC_PARAMS_COUNT: usize = 6;
/// Total size of the static part of the finalizeBridgeToken ABI call
pub const ABI_FINALIZE_BRIDGE_STATIC_PART_SIZE: usize =
    ABI_FINALIZE_BRIDGE_STATIC_PARAMS_COUNT * ABI_ADDRESS_PARAM_SIZE;

/// Parameters for initiating a token bridge transaction.
/// Contains all necessary data to bridge tokens from Solana to Base.
struct BridgeCallParams {
    /// Address of the corresponding ERC20 token on Base
    remote_token: [u8; 20],
    /// Address of the receiver on Base
    to: [u8; 20],
    /// Amount of tokens to bridge
    amount: u64,
    /// Minimum gas limit for the transaction on Base
    min_gas_limit: u32,
    /// Additional data to include with the bridge transaction
    extra_data: Vec<u8>,
}

/// Accounts required for bridging SOL to Base.
///
/// This instruction transfers native SOL from the user to a vault PDA
/// and sends a message to Base to mint the corresponding wrapped SOL.
#[derive(Accounts)]
#[instruction(remote_token: [u8; 20])]
pub struct BridgeSolTo<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK: This is the hardcoded gas fee receiver account.
    #[account(mut, address = GAS_FEE_RECEIVER)]
    pub gas_fee_receiver: AccountInfo<'info>,

    pub system_program: Program<'info, System>,

    #[account(mut, seeds = [MESSENGER_SEED], bump = messenger.bump)]
    pub messenger: Account<'info, Messenger>,

    /// CHECK: This is the sol vault account for a specific remote token.
    #[account(mut, seeds = [SOL_VAULT_SEED, remote_token.as_ref()], bump)]
    pub sol_vault: AccountInfo<'info>,
}

/// Accounts required for bridging SPL tokens to Base.
///
/// This instruction either burns tokens (if owned by bridge) or transfers them
/// to a vault, then sends a message to Base to mint the corresponding ERC20.
#[derive(Accounts)]
#[instruction(remote_token: [u8; 20])]
pub struct BridgeTokensTo<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK: This is the hardcoded gas fee receiver account.
    #[account(mut, address = GAS_FEE_RECEIVER)]
    pub gas_fee_receiver: AccountInfo<'info>,

    pub system_program: Program<'info, System>,

    #[account(mut, seeds = [MESSENGER_SEED], bump = messenger.bump)]
    pub messenger: Account<'info, Messenger>,

    #[account(mut)]
    pub mint: Account<'info, Mint>,

    #[account(mut)]
    pub from_token_account: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        seeds = [TOKEN_VAULT_SEED, mint.key().as_ref(), remote_token.as_ref()],
        bump,
        payer = user,
        token::mint = mint,
        token::authority = token_vault
    )]
    pub token_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

/// Event emitted when a token bridge is initiated from Solana to Base.
///
/// This event contains all the information needed to track the bridge
/// transaction and process it on the destination chain.
#[event]
pub struct TokenBridgeInitiated {
    /// Address of the token on Solana
    pub local_token: Pubkey,
    /// Address of the corresponding ERC20 token on Base
    pub remote_token: [u8; 20],
    /// Address of the sender on Solana
    pub from: Pubkey,
    /// Address of the receiver on Base
    pub to: [u8; 20],
    /// Amount of tokens being bridged
    pub amount: u64,
    /// Additional data sent with the transaction
    pub extra_data: Vec<u8>,
}

/// Event emitted when a token bridge is finalized on Solana from Base.
///
/// This event is emitted when tokens are successfully received and
/// distributed on Solana from a bridge transaction originating on Base.
#[event]
pub struct TokenBridgeFinalized {
    /// Address of the token on Solana
    pub local_token: Pubkey,
    /// Address of the corresponding ERC20 token on Base
    pub remote_token: [u8; 20],
    /// Address of the sender on Base
    pub from: [u8; 20],
    /// Address of the receiver on Solana
    pub to: Pubkey,
    /// Amount of tokens being bridged
    pub amount: u64,
    /// Additional data sent with the transaction
    pub extra_data: Vec<u8>,
}

/// Bridges native SOL to a receiver's address on Base.
///
/// This function transfers SOL from the user to a vault PDA and sends a message
/// to the Base TokenBridge contract to mint the corresponding wrapped SOL.
///
/// # Arguments
/// * `remote_token`  - Address of the corresponding token on Base
/// * `to`            - Address of the receiver on Base
/// * `amount`        - Amount of SOL to bridge (in lamports)
/// * `min_gas_limit` - Minimum gas limit for the cross-chain transaction
/// * `extra_data`    - Additional data to include with the transaction
pub fn bridge_sol_to_handler(
    ctx: Context<BridgeSolTo>,
    remote_token: [u8; 20],
    to: [u8; 20],
    amount: u64,
    min_gas_limit: u32,
    extra_data: Vec<u8>,
) -> Result<()> {
    // Transfer lamports from user to vault PDA
    let cpi_context = CpiContext::new(
        ctx.accounts.system_program.to_account_info(),
        anchor_lang::system_program::Transfer {
            from: ctx.accounts.user.to_account_info(),
            to: ctx.accounts.sol_vault.to_account_info(),
        },
    );
    anchor_lang::system_program::transfer(cpi_context, amount)?;

    emit_event_and_send_message(
        &ctx.accounts.system_program,
        &ctx.accounts.gas_fee_receiver,
        &ctx.accounts.user,
        &mut ctx.accounts.messenger,
        NATIVE_SOL_PUBKEY,
        &BridgeCallParams {
            remote_token,
            to,
            amount,
            min_gas_limit,
            extra_data,
        },
    )
}

/// Bridges SPL tokens to a receiver's address on Base.
///
/// This function either burns tokens (if they are owned by the bridge) or
/// transfers them to a vault, then sends a message to the Base TokenBridge
/// contract to mint the corresponding ERC20 tokens.
///
/// # Arguments
/// * `remote_token`  - Address of the corresponding ERC20 token on Base
/// * `to`            - Address of the receiver on Base
/// * `amount`        - Amount of tokens to bridge
/// * `min_gas_limit` - Minimum gas limit for the cross-chain transaction
/// * `extra_data`    - Additional data to include with the transaction
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

    if is_owned_by_bridge(&ctx.accounts.mint.to_account_info())? {
        // Burn tokens if the mint is owned by the bridge
        let cpi_context = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Burn {
                mint: ctx.accounts.mint.to_account_info(),
                from: ctx.accounts.from_token_account.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );
        anchor_spl::token::burn(cpi_context, amount)?;
    } else {
        // Transfer tokens to vault if the mint is not owned by the bridge
        let cpi_accounts = anchor_spl::token::Transfer {
            from: ctx.accounts.from_token_account.to_account_info(),
            to: ctx.accounts.token_vault.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        anchor_spl::token::transfer(cpi_ctx, amount)?;
    }

    emit_event_and_send_message(
        &ctx.accounts.system_program,
        &ctx.accounts.gas_fee_receiver,
        &ctx.accounts.user,
        &mut ctx.accounts.messenger,
        ctx.accounts.mint.key(),
        &BridgeCallParams {
            remote_token,
            to,
            amount,
            min_gas_limit,
            extra_data,
        },
    )
}

/// Finalizes a token bridge transaction originating from Base.
///
/// This function can only be called by the remote TokenBridge contract on Base
/// through the cross-chain messaging system. It handles the distribution of tokens
/// on Solana by either unlocking them from vaults or minting new tokens.
///
/// # Arguments
/// * `message`            - The cross-chain message containing bridge details
/// * `remaining_accounts` - Additional accounts needed for token operations
/// * `payload`            - Decoded bridge payload with token transfer details
pub fn finalize_bridge_tokens<'info>(
    message: &mut Account<'info, Message>,
    remaining_accounts: &'info [AccountInfo<'info>],
    payload: BridgePayload,
) -> Result<()> {
    require!(
        message.messenger_caller == REMOTE_BRIDGE,
        BridgeError::OnlyRemoteBridgeCanCall
    );

    // Check if the bridge is paused (L1 only, no-op on L2)
    require!(!messenger::paused(), MessengerError::BridgeIsPaused);

    let to = find_account_info_by_key(remaining_accounts, &payload.to)?;

    if payload.local_token == NATIVE_SOL_PUBKEY {
        // Handle native SOL bridge finalization
        unlock_sol_from_vault(
            remaining_accounts,
            to,
            &payload.remote_token,
            payload.amount,
        )?;
    } else {
        let token_program_info = find_account_info_by_key(remaining_accounts, &TOKEN_PROGRAM_ID)?;
        let mint_info = find_account_info_by_key(remaining_accounts, &payload.local_token)?;

        if is_owned_by_bridge(mint_info)? {
            // Mint new tokens if the mint is owned by the bridge
            mint_to_recipient(
                token_program_info,
                mint_info,
                to,
                &payload.remote_token,
                payload.amount,
            )?;
        } else {
            // Unlock tokens from vault if the mint is external
            unlock_tokens_from_vault(
                token_program_info,
                mint_info,
                to,
                remaining_accounts,
                &payload.remote_token,
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

/// Emits a bridge event and sends a cross-chain message to Base.
///
/// This helper function handles the common logic for both SOL and token bridging:
/// emitting the appropriate event and sending the encoded message to the remote bridge.
///
/// # Arguments
/// * `system_program`   - Solana system program
/// * `gas_fee_receiver` - Account to receive gas fees
/// * `user`             - User initiating the bridge
/// * `messenger`        - Messenger account for cross-chain communication
/// * `mint`             - Token mint (or native SOL pubkey)
/// * `bridge_params`    - Bridge transaction parameters
fn emit_event_and_send_message<'info>(
    system_program: &Program<'info, System>,
    gas_fee_receiver: &AccountInfo<'info>,
    user: &Signer<'info>,
    messenger: &mut Account<'info, Messenger>,
    mint: Pubkey,
    bridge_params: &BridgeCallParams,
) -> Result<()> {
    emit!(TokenBridgeInitiated {
        local_token: mint,
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
            mint,
            user.key(),
            bridge_params.to,
            bridge_params.amount,
            &bridge_params.extra_data,
        ),
        bridge_params.min_gas_limit,
    )
}

/// Encodes the ABI call for `Base.TokenBridge.finalizeBridgeToken`.
///
/// This function creates a properly formatted ABI-encoded payload that can be
/// executed on the Base TokenBridge contract to complete the bridge transaction.
///
/// # Arguments
/// * `remote_token_on_base`  - Address of the ERC20 token on Base
/// * `local_token_on_solana` - Address of the token on Solana
/// * `from_on_solana`        - Address of the sender on Solana
/// * `to_on_base`            - Address of the receiver on Base
/// * `amount`                - Amount of tokens being bridged
/// * `extra_data`            - Additional data to include
///
/// # Returns
/// A byte vector containing the ABI-encoded function call
fn encode_finalize_brigde_token_call(
    remote_token_on_base: [u8; 20],
    local_token_on_solana: Pubkey,
    from_on_solana: Pubkey,
    to_on_base: [u8; 20],
    amount: u64,
    extra_data: &[u8],
) -> Vec<u8> {
    let mut encoded_payload = Vec::new();

    // Function selector for Base.TokenBridge.finalizeBridgeToken: 0x2d916920
    encoded_payload.extend_from_slice(&FINALIZE_BRIDGE_TOKEN_SELECTOR);

    // Parameter 1: remoteToken (address, 32-byte padded)
    append_padded_abi_bytes(
        &mut encoded_payload,
        &remote_token_on_base,
        ABI_ADDRESS_PARAM_SIZE,
    );

    // Parameter 2: localToken (address, 32-byte padded)
    append_padded_abi_bytes(
        &mut encoded_payload,
        local_token_on_solana.as_ref(),
        ABI_ADDRESS_PARAM_SIZE,
    );

    // Parameter 3: _from (address, 32-byte padded)
    append_padded_abi_bytes(
        &mut encoded_payload,
        from_on_solana.as_ref(),
        ABI_ADDRESS_PARAM_SIZE,
    );

    // Parameter 4: _to (address, 32-byte padded)
    append_padded_abi_bytes(&mut encoded_payload, &to_on_base, ABI_ADDRESS_PARAM_SIZE);

    // Parameter 5: _amount (uint256, 32-byte padded)
    append_padded_abi_bytes(
        &mut encoded_payload,
        &amount.to_be_bytes(),
        ABI_U64_PARAM_SIZE,
    );

    // Parameter 6: _extraData offset (points to dynamic data location)
    append_padded_abi_bytes(
        &mut encoded_payload,
        &(ABI_FINALIZE_BRIDGE_STATIC_PART_SIZE as u64).to_be_bytes(),
        ABI_DYNAMIC_OFFSET_SIZE,
    );

    // Dynamic part: extraData length
    append_padded_abi_bytes(
        &mut encoded_payload,
        &(extra_data.len() as u64).to_be_bytes(),
        ABI_U64_PARAM_SIZE,
    );

    // Dynamic part: extraData content
    encoded_payload.extend_from_slice(extra_data);

    // Pad extraData to 32-byte boundary for ABI compliance
    let padding_len = (ABI_ADDRESS_PARAM_SIZE - (extra_data.len() % ABI_ADDRESS_PARAM_SIZE))
        % ABI_ADDRESS_PARAM_SIZE;
    encoded_payload.extend_from_slice(&vec![0u8; padding_len]);

    encoded_payload
}

/// Pads data to a specific ABI length and appends it to the encoded vector.
///
/// This helper function ensures proper ABI encoding by padding data to the required
/// length (typically 32 bytes) with zero bytes at the beginning (right-alignment).
///
/// # Arguments
/// * `encoded_vec` - The vector to append the padded data to
/// * `data_slice`  - The data to pad and append
/// * `abi_length`  - The required padded length (typically 32 bytes)
fn append_padded_abi_bytes(encoded_vec: &mut Vec<u8>, data_slice: &[u8], abi_length: usize) {
    let mut padded_data = vec![0u8; abi_length];
    let data_len = data_slice.len();
    if data_len > abi_length {
        // For oversized data, truncate by taking the rightmost bytes
        // This handles cases where data needs to fit into the ABI parameter size
        padded_data[(abi_length - data_len)..].copy_from_slice(data_slice);
    } else {
        // Pad at the beginning (right-align for ABI compliance)
        padded_data[(abi_length - data_len)..].copy_from_slice(data_slice);
    }
    encoded_vec.extend_from_slice(&padded_data);
}

/// Checks if a token mint is owned and controlled by the bridge program.
///
/// A mint is considered "owned by bridge" if:
/// 1. It's a valid SPL Token mint
/// 2. It's initialized
/// 3. Its mint authority is set to itself (indicating it's a PDA controlled by the bridge)
///
/// # Arguments
/// * `mint_info` - The account info for the token mint
fn is_owned_by_bridge(mint_info: &AccountInfo<'_>) -> Result<bool> {
    // Verify the account is owned by the SPL Token program
    if *mint_info.owner != TOKEN_PROGRAM_ID {
        return Ok(false);
    }

    // Deserialize the mint data to check its properties
    let mint = anchor_spl::token::Mint::try_deserialize(&mut &mint_info.try_borrow_data()?[..])?;

    // Check if mint is initialized and its authority is itself (PDA pattern)
    Ok(mint.is_initialized && mint.mint_authority == COption::Some(mint_info.key()))
}

/// Unlocks native SOL from a vault PDA and transfers it to the recipient.
///
/// This function is used during bridge finalization to release SOL that was
/// previously locked in a vault when bridging from Solana to Base.
///
/// # Arguments
/// * `remaining_accounts` - Array of additional accounts needed for the operation
/// * `to`                 - The recipient account for the SOL
/// * `remote_token`       - The remote token identifier used in vault derivation
/// * `amount`             - Amount of SOL to transfer (in lamports)
fn unlock_sol_from_vault<'info>(
    remaining_accounts: &'info [AccountInfo<'info>],
    to: &AccountInfo<'info>,
    remote_token: &[u8; 20],
    amount: u64,
) -> Result<()> {
    let system_program = find_account_info_by_key(remaining_accounts, &System::id())?;

    // Derive the SOL vault PDA
    let (sol_vault_pubkey, sol_vault_bump) =
        Pubkey::find_program_address(&[SOL_VAULT_SEED, remote_token.as_ref()], &crate::ID);
    let bump_array = [sol_vault_bump];

    // Prepare seeds for PDA signing
    let sol_vault_seeds: &[&[u8]] = &[SOL_VAULT_SEED, remote_token.as_ref()];
    let mut seeds_and_bump: Vec<&[u8]> = Vec::with_capacity(sol_vault_seeds.len() + 1);
    seeds_and_bump.extend_from_slice(sol_vault_seeds);
    seeds_and_bump.push(&bump_array);
    let seeds_and_bump: &[&[&[u8]]] = &[&seeds_and_bump];

    let sol_vault_info = find_account_info_by_key(remaining_accounts, &sol_vault_pubkey)?;

    // Transfer SOL from vault to recipient using PDA signature
    let cpi_context = CpiContext::new_with_signer(
        system_program.to_account_info(),
        anchor_lang::system_program::Transfer {
            from: sol_vault_info.to_account_info(),
            to: to.to_account_info(),
        },
        seeds_and_bump,
    );
    anchor_lang::system_program::transfer(cpi_context, amount)
}

/// Unlocks SPL tokens from a vault PDA and transfers them to the recipient.
///
/// This function is used during bridge finalization to release tokens that were
/// previously locked in a vault when bridging from Solana to Base.
///
/// # Arguments
/// * `token_program_info` - The SPL Token program account
/// * `mint_info`          - The token mint account
/// * `to`                 - The recipient token account
/// * `remaining_accounts` - Array of additional accounts needed for the operation
/// * `remote_token`       - The remote token identifier used in vault derivation
/// * `amount`             - Amount of tokens to transfer
fn unlock_tokens_from_vault<'info>(
    token_program_info: &'info AccountInfo<'info>,
    mint_info: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    remaining_accounts: &'info [AccountInfo<'info>],
    remote_token: &[u8; 20],
    amount: u64,
) -> Result<()> {
    let mint_info_key = mint_info.key.to_bytes();

    // Derive the token vault PDA
    let token_vault_seeds: &[&[u8]] = &[
        TOKEN_VAULT_SEED,
        mint_info_key.as_ref(),
        remote_token.as_ref(),
    ];
    let (token_vault_key, token_vault_bump) =
        Pubkey::find_program_address(token_vault_seeds, &crate::ID);
    let token_vault_info = find_account_info_by_key(remaining_accounts, &token_vault_key)?;
    let bump_array = [token_vault_bump];

    // Prepare seeds for PDA signing
    let mut seeds_and_bump: Vec<&[u8]> = Vec::with_capacity(token_vault_seeds.len() + 1);
    seeds_and_bump.extend_from_slice(token_vault_seeds);
    seeds_and_bump.push(&bump_array);
    let seeds_and_bump: &[&[&[u8]]] = &[&seeds_and_bump];

    // Transfer tokens from vault to recipient using PDA signature
    let cpi_ctx = CpiContext::new(
        token_program_info.clone(),
        anchor_spl::token::Transfer {
            from: token_vault_info.clone(),
            to: to.clone(),
            authority: token_vault_info.clone(),
        },
    )
    .with_signer(seeds_and_bump);

    anchor_spl::token::transfer(cpi_ctx, amount)
}

/// Mints new tokens to the recipient account.
///
/// This function is used when finalizing bridge transactions for tokens that are
/// owned by the bridge program. Instead of unlocking from a vault, new tokens are
/// minted directly to the recipient.
///
/// # Arguments
/// * `token_program_info` - The SPL Token program account
/// * `mint_info`          - The token mint account (must be owned by bridge)
/// * `to`                 - The recipient token account
/// * `remote_token`       - The remote token identifier used in mint derivation
/// * `amount`             - Amount of tokens to mint
fn mint_to_recipient<'info>(
    token_program_info: &'info AccountInfo<'info>,
    mint_info: &'info AccountInfo<'info>,
    to: &AccountInfo<'info>,
    remote_token: &[u8; 20],
    amount: u64,
) -> Result<()> {
    let mint_account = Account::<Mint>::try_from(mint_info)?;
    let decimals_bytes = mint_account.decimals.to_le_bytes(); // TODO: Fix this when we correctly implement decimals clamping.

    // Derive the expected mint PDA
    let seeds: &[&[u8]] = &[MINT_SEED, remote_token.as_ref(), decimals_bytes.as_ref()];
    let (mint_key, bump_value) = Pubkey::find_program_address(seeds, &crate::ID);
    require_keys_eq!(mint_key, mint_info.key(), BridgeError::InvalidTokenPair);
    let bump_slice = [bump_value];

    // Prepare seeds for PDA signing
    let mut seeds_and_bump: Vec<&[u8]> = Vec::with_capacity(seeds.len() + 1);
    seeds_and_bump.extend_from_slice(seeds);
    seeds_and_bump.push(&bump_slice);
    let seeds_and_bump: &[&[&[u8]]] = &[&seeds_and_bump];

    // Mint tokens to recipient using PDA authority
    let cpi_context = CpiContext::new(
        token_program_info.clone(),
        MintTo {
            mint: mint_info.clone(),
            to: to.clone(),
            authority: mint_info.clone(),
        },
    )
    .with_signer(seeds_and_bump);
    token_interface::mint_to(cpi_context, amount)
}

/// Finds an AccountInfo by its public key from a slice of AccountInfos.
///
/// This helper function searches through the remaining accounts to find one
/// with a matching public key. This is commonly used in Solana programs when
/// working with variable numbers of accounts.
///
/// # Arguments
/// * `remaining_accounts` - The slice of AccountInfo structs to search through
/// * `key`                - The public key to search for
///
/// # Note
/// TODO: Consider forcing accounts to be at specific indices instead of searching
/// for better performance and predictability.
fn find_account_info_by_key<'a, 'info>(
    remaining_accounts: &'a [AccountInfo<'info>],
    key: &Pubkey,
) -> Result<&'a AccountInfo<'info>> {
    remaining_accounts
        .iter()
        .find(|acc_info| acc_info.key == key)
        .ok_or(ProgramError::NotEnoughAccountKeys.into())
}

/// Derives the local bridge program's public key.
///
/// This function creates a deterministic public key that represents this bridge
/// program on the local chain. It's used as the sender address when sending
/// cross-chain messages to Base.
///
/// The key is derived using: keccak256(abi.encodePacked(programId, "bridge"))
pub fn local_bridge_pubkey() -> Pubkey {
    let mut data_to_hash = Vec::new();
    data_to_hash.extend_from_slice(crate::ID.as_ref());
    data_to_hash.extend_from_slice(BRIDGE_SEED);
    let hash = keccak::hash(&data_to_hash);
    Pubkey::new_from_array(hash.to_bytes())
}

/// Errors that can occur during bridge operations.
#[error_code]
pub enum BridgeError {
    /// Attempted to use native SOL in a context where it's not allowed
    #[msg("Cannot bridge SOL here")]
    InvalidSolUsage,
    /// Attempted to call a function that's restricted to the remote bridge
    #[msg("Only remote bridge can call")]
    OnlyRemoteBridgeCanCall,
    /// Insufficient balance for the requested operation
    #[msg("Insufficient balance")]
    InsufficientBalance,
    /// The provided token pair doesn't match expected values
    #[msg("Invalid token pair")]
    InvalidTokenPair,
}
