use anchor_lang::{
    prelude::*,
    solana_program::{self, instruction::Instruction},
};

use crate::{
    ix_utils, merkle_utils, Ix, Message, MessengerPayload, OutputRoot, DEFAULT_SENDER,
    MESSAGE_SEED, VAULT_SEED, VERSION,
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
#[instruction(transaction_hash: [u8; 32])]
pub struct FinalizeTransaction<'info> {
    #[account(mut, seeds = [MESSAGE_SEED, &transaction_hash], bump)]
    pub message: Account<'info, Message>,

    /// CHECK: This is the vault PDA. For SOL, it receives SOL. For SPL, it's the authority for vault_token_account.
    #[account(
        mut,
        seeds = [VAULT_SEED, VERSION.to_le_bytes().as_ref()],
        bump
    )]
    pub vault: AccountInfo<'info>,
}

pub fn prove_transaction_handler(
    ctx: Context<ProveTransaction>,
    transaction_hash: &[u8; 32],
    remote_sender: &[u8; 20],
    ixs: Vec<Ix>,
    proof: Vec<[u8; 32]>,
    leaf_index: u64,
    total_leaf_count: u64,
) -> Result<()> {
    let message_hash = ix_utils::hash_ixs(remote_sender, &ixs);

    if message_hash != *transaction_hash {
        return err!(ReceiverError::InvalidTransactionHash);
    }

    // Run merkle proof of proof against ctx.accounts.root.root
    if !merkle_utils::verify_mmr_proof(
        &proof,
        &ctx.accounts.root.root,
        transaction_hash,
        leaf_index,
        total_leaf_count,
    ) {
        return err!(ReceiverError::InvalidProof);
    }

    ctx.accounts.message.ixs = ixs;
    ctx.accounts.message.sender = DEFAULT_SENDER;
    ctx.accounts.message.remote_sender = *remote_sender;

    Ok(())
}

pub fn finalize_transaction_handler<'a, 'info>(
    ctx: Context<'a, '_, 'info, 'info, FinalizeTransaction<'info>>,
    _transaction_hash: &[u8; 32],
) -> Result<()> {
    if ctx.accounts.message.is_executed {
        return err!(ReceiverError::AlreadyExecuted);
    }

    ctx.accounts.message.is_executed = true;
    handle_ixs(
        ctx.remaining_accounts,
        &mut ctx.accounts.message,
        &ctx.accounts.vault,
    )
}

fn handle_ixs<'info>(
    account_infos: &'info [AccountInfo<'info>],
    message_account: &mut Account<'info, Message>,
    vault: &AccountInfo<'info>,
) -> Result<()> {
    // Clone `ixs` because `messenger::relay_message` requires a mutable borrow of `message_account`,
    // which would conflict with an immutable borrow for iterating `message_account.ixs` directly.
    for ix_data in &message_account.ixs.clone() {
        let ix: Instruction = ix_data.into();
        if ix.program_id == messenger::local_messenger_pubkey() {
            messenger::relay_message(
                message_account,
                vault,
                account_infos,
                &message_account.remote_sender.clone(),
                MessengerPayload::try_from_slice(&ix.data)?,
            )?;
        } else {
            solana_program::program::invoke(&ix, account_infos)?;
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
