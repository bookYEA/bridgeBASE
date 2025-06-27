use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::{
    common::TOKEN_VAULT_SEED,
    solana_to_base::{
        process_spl_transfer_operation, Operation, OutgoingMessageHeader, Transfer as TransferOp,
        MESSAGE_HEADER_SEED, OPERATION_SEED,
    },
};

#[derive(Accounts)]
#[instruction(id: u64, remote_token: [u8; 20])]
pub struct BufferSplTransfer<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub from: Signer<'info>,

    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub from_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [TOKEN_VAULT_SEED, remote_token.as_ref()],
        bump,
    )]
    pub token_vault: InterfaceAccount<'info, TokenAccount>,

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
    pub transfer_spl_operation: Account<'info, Operation>,

    pub token_program: Interface<'info, TokenInterface>,

    pub system_program: Program<'info, System>,
}

pub fn buffer_spl_transfer_handler(
    ctx: Context<BufferSplTransfer>,
    _id: u64,
    gas_limit: u64,
    to: [u8; 20],
    remote_token: [u8; 20],
    amount: u64,
) -> Result<()> {
    process_spl_transfer_operation(
        &ctx.accounts.mint,
        &ctx.accounts.from_token_account,
        &ctx.accounts.token_vault,
        &ctx.accounts.from,
        &ctx.accounts.token_program,
        ctx.accounts.outgoing_message_header.gas_limit + gas_limit,
        amount,
    )?;

    // Create the transfer operation.
    *ctx.accounts.transfer_spl_operation = Operation::new_transfer(TransferOp {
        to,
        local_token: ctx.accounts.mint.key(),
        remote_token,
        amount,
    });

    // Update the message header.
    ctx.accounts.outgoing_message_header.gas_limit += gas_limit;
    ctx.accounts.outgoing_message_header.operation_count += 1;

    Ok(())
}
