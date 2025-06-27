use anchor_lang::prelude::*;

use crate::{
    common::SOL_VAULT_SEED,
    solana_to_base::{
        process_sol_transfer_operation, Operation, OutgoingMessageHeader, Transfer as TransferOp,
        MESSAGE_HEADER_SEED, NATIVE_SOL_PUBKEY, OPERATION_SEED,
    },
};

#[derive(Accounts)]
#[instruction(id: u64, remote_token: [u8; 20])]
pub struct BufferSolTransfer<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub from: Signer<'info>,

    /// CHECK: This is the SOL vault account.
    #[account(
        mut,
        seeds = [SOL_VAULT_SEED, remote_token.as_ref()],
        bump,
    )]
    pub sol_vault: AccountInfo<'info>,

    #[account(
        init_if_needed,
        seeds = [MESSAGE_HEADER_SEED, from.key().as_ref(), id.to_le_bytes().as_ref()],
        bump,
        payer = payer,
        space = 8 + OutgoingMessageHeader::INIT_SPACE,
    )]
    pub outgoing_message_header: Account<'info, OutgoingMessageHeader>,

    #[account(
        init,
        seeds = [
            OPERATION_SEED,
            from.key().as_ref(), 
            id.to_le_bytes().as_ref(),
            outgoing_message_header.operation_count.to_le_bytes().as_ref(),
        ],
        bump,
        payer = payer,
        space = 8 + Operation::transfer_space(),
    )]
    pub sol_transfer_operation: Account<'info, Operation>,

    pub system_program: Program<'info, System>,
}

pub fn buffer_sol_transfer_handler(
    ctx: Context<BufferSolTransfer>,
    _id: u64,
    gas_limit: u64,
    to: [u8; 20],
    remote_token: [u8; 20],
    amount: u64,
) -> Result<()> {
    process_sol_transfer_operation(
        ctx.accounts.sol_vault.to_account_info(),
        ctx.accounts.from.to_account_info(),
        &ctx.accounts.system_program,
        ctx.accounts.outgoing_message_header.gas_limit + gas_limit,
        amount,
    )?;

    // Create the transfer operation.
    *ctx.accounts.sol_transfer_operation = Operation::new_transfer(TransferOp {
        to,
        local_token: NATIVE_SOL_PUBKEY,
        remote_token,
        amount,
    });

    // Update the message header.
    ctx.accounts.outgoing_message_header.gas_limit += gas_limit;
    ctx.accounts.outgoing_message_header.operation_count += 1;

    Ok(())
}
