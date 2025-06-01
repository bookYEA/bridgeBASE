use anchor_lang::{
    prelude::*,
    solana_program::{self},
};

use crate::{
    ix_utils, merkle_utils, Ix, Message, MessengerPayload, OutputRoot, DEFAULT_MESSENGER_CALLER,
    MESSAGE_SEED,
};

use super::messenger;

/// Account structure for proving a cross-chain transaction
///
/// This instruction creates a new Message account that stores the transaction data
/// after verifying the MMR proof against the stored output root.
#[derive(Accounts)]
#[instruction(transaction_hash: [u8; 32])]
pub struct ProveTransaction<'info> {
    /// The message account to be created, seeded by the transaction hash
    /// This will store the proven transaction data for later execution
    #[account(
        init,
        payer = payer,
        space = 8 + Message::INIT_SPACE,
        seeds = [MESSAGE_SEED, &transaction_hash],
        bump
    )]
    pub message: Account<'info, Message>,

    /// The output root account containing the MMR root to verify against
    /// This represents a committed state from Base
    pub root: Account<'info, OutputRoot>,

    /// The account paying for the message account creation
    #[account(mut)]
    payer: Signer<'info>,

    /// System program for account creation
    pub system_program: Program<'info, System>,
}

/// Account structure for finalizing a proven transaction
///
/// This instruction executes the instructions stored in a previously proven message.
#[derive(Accounts)]
pub struct FinalizeTransaction<'info> {
    /// The proven message account containing instructions to execute
    /// Must not have been executed previously
    #[account(mut)]
    pub message: Account<'info, Message>,
}

/// Proves a cross-chain transaction by verifying its inclusion in the source chain
///
/// This function verifies that a transaction exists on Base by checking
/// an MMR proof against a stored output root. If valid, it stores the transaction
/// data in a Message account for later execution.
///
/// # Arguments
/// * `ctx`                   - The transaction context containing accounts
/// * `transaction_hash`      - Hash of the transaction to prove
/// * `nonce`                 - Unique nonce used in the original transaction
/// * `message_passer_caller` - Address that initiated the cross-chain message
/// * `ixs`                   - Vector of instructions to be executed on finalization
/// * `proof`                 - MMR proof demonstrating transaction inclusion
/// * `leaf_index`            - Position of the transaction in the merkle tree
/// * `total_leaf_count`      - Total number of leaves in the merkle tree
#[allow(clippy::too_many_arguments)]
pub fn prove_transaction_handler(
    ctx: Context<ProveTransaction>,
    transaction_hash: &[u8; 32],
    nonce: &[u8; 32],
    message_passer_caller: &[u8; 20],
    ixs: Vec<Ix>,
    proof: Vec<[u8; 32]>,
    leaf_index: u64,
    total_leaf_count: u64,
) -> Result<()> {
    // Verify the transaction hash matches the computed hash of the instructions
    let message_hash = ix_utils::hash_ixs(nonce, message_passer_caller, &ixs);
    require!(
        message_hash == *transaction_hash,
        ReceiverError::InvalidTransactionHash
    );

    // Verify the merkle proof to ensure the transaction exists on the source chain
    require!(
        merkle_utils::verify_mmr_proof(
            &proof,
            &ctx.accounts.root.root,
            transaction_hash,
            leaf_index,
            total_leaf_count,
        ),
        ReceiverError::InvalidProof
    );

    // Store the proven transaction data in the message account
    ctx.accounts.message.ixs = ixs;
    ctx.accounts.message.message_passer_caller = *message_passer_caller;
    ctx.accounts.message.messenger_caller = DEFAULT_MESSENGER_CALLER;

    Ok(())
}

/// Finalizes a proven cross-chain transaction by executing its instructions
///
/// This function executes the instructions stored in a previously proven message.
/// It marks the message as executed to prevent replay attacks and handles both
/// regular program instructions and special messenger instructions.
///
/// # Arguments
/// * `ctx` - The transaction context containing the message account and remaining accounts
pub fn finalize_transaction_handler<'a, 'info>(
    ctx: Context<'a, '_, 'info, 'info, FinalizeTransaction<'info>>,
) -> Result<()> {
    // Ensure the message hasn't been executed already (replay protection)
    require!(
        !ctx.accounts.message.is_executed,
        ReceiverError::AlreadyExecuted
    );

    // Mark the message as executed
    ctx.accounts.message.is_executed = true;

    // Execute all instructions in the message
    handle_ixs(ctx.remaining_accounts, &mut ctx.accounts.message)
}

/// Handles execution of instructions stored in a cross-chain message
///
/// This function iterates through all instructions in the message and executes them.
/// It provides special handling for messenger instructions by calling the messenger
/// relay function, while other instructions are executed using standard program invoke.
///
/// # Arguments
/// * `remaining_accounts` - Array of account infos needed for instruction execution
/// * `message`            - The message containing instructions to execute
fn handle_ixs<'info>(
    remaining_accounts: &'info [AccountInfo<'info>],
    message: &mut Account<'info, Message>,
) -> Result<()> {
    // Clone `ixs` because `messenger::relay_message` requires a mutable borrow of `message`,
    // which would conflict with an immutable borrow for iterating `message.ixs` directly.
    for ix in &message.ixs.clone() {
        if ix.program_id == messenger::local_messenger_pubkey() {
            // Handle messenger instructions with special relay logic
            messenger::relay_message(
                message,
                remaining_accounts,
                MessengerPayload::try_from_slice(&ix.data)?,
                true,
            )?;
        } else {
            // Execute regular program instructions
            solana_program::program::invoke(&ix.into(), remaining_accounts)?;
        }
    }
    Ok(())
}

/// Error codes for cross-chain message receiver operations
#[error_code]
pub enum ReceiverError {
    /// Thrown when the provided transaction hash doesn't match the computed hash
    #[msg("Invalid transaction hash")]
    InvalidTransactionHash,
    /// Thrown when the MMR proof verification fails
    #[msg("Invalid proof")]
    InvalidProof,
    /// Thrown when attempting to execute a message that has already been executed
    #[msg("Already executed")]
    AlreadyExecuted,
}
