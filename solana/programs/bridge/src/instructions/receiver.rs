use anchor_lang::{
    prelude::*,
    solana_program::{self},
};

use crate::{
    ix_utils, merkle_utils, Ix, Message, MessengerPayload, OutputRoot, DEFAULT_MESSENGER_CALLER,
    MESSAGE_SEED,
};

use super::messenger;

#[derive(Accounts)]
#[instruction(transaction_hash: [u8; 32])]
pub struct ProveTransaction<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + Message::INIT_SPACE,
        seeds = [MESSAGE_SEED, &transaction_hash],
        bump
    )]
    pub message: Account<'info, Message>,

    pub root: Account<'info, OutputRoot>,

    #[account(mut)]
    payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct FinalizeTransaction<'info> {
    #[account(mut)]
    pub message: Account<'info, Message>,
}

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
    let message_hash = ix_utils::hash_ixs(nonce, message_passer_caller, &ixs);
    require!(
        message_hash == *transaction_hash,
        ReceiverError::InvalidTransactionHash
    );

    // Run merkle proof of proof against ctx.accounts.root.root
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

    ctx.accounts.message.ixs = ixs;
    ctx.accounts.message.message_passer_caller = *message_passer_caller;
    ctx.accounts.message.messenger_caller = DEFAULT_MESSENGER_CALLER;

    Ok(())
}

pub fn finalize_transaction_handler<'a, 'info>(
    ctx: Context<'a, '_, 'info, 'info, FinalizeTransaction<'info>>,
) -> Result<()> {
    require!(
        !ctx.accounts.message.is_executed,
        ReceiverError::AlreadyExecuted
    );

    ctx.accounts.message.is_executed = true;
    handle_ixs(ctx.remaining_accounts, &mut ctx.accounts.message)
}

fn handle_ixs<'info>(
    remaining_accounts: &'info [AccountInfo<'info>],
    message: &mut Account<'info, Message>,
) -> Result<()> {
    // Clone `ixs` because `messenger::relay_message` requires a mutable borrow of `message`,
    // which would conflict with an immutable borrow for iterating `message.ixs` directly.
    for ix in &message.ixs.clone() {
        if ix.program_id == messenger::local_messenger_pubkey() {
            messenger::relay_message(
                message,
                remaining_accounts,
                MessengerPayload::try_from_slice(&ix.data)?,
                true,
            )?;
        } else {
            solana_program::program::invoke(&ix.into(), remaining_accounts)?;
        }
    }
    Ok(())
}

#[error_code]
pub enum ReceiverError {
    #[msg("Invalid transaction hash")]
    InvalidTransactionHash,
    #[msg("Invalid proof")]
    InvalidProof,
    #[msg("Already executed")]
    AlreadyExecuted,
}
