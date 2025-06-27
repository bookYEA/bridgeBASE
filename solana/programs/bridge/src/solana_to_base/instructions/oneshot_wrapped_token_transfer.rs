use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::Token2022,
    token_interface::{Mint, TokenAccount},
};

use crate::{
    common::{bridge::Bridge, BRIDGE_SEED},
    solana_to_base::{
        pay_for_gas, process_wrapped_token_transfer_operation, Operation, OutgoingMessage,
        Transfer as TransferOp, GAS_FEE_RECEIVER, OUTGOING_MESSAGE_SEED,
    },
};

#[derive(Accounts)]
pub struct OneshotWrappedTokenTransfer<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub from: Signer<'info>,

    /// CHECK: This is the hardcoded gas fee receiver account.
    #[account(mut, address = GAS_FEE_RECEIVER @ OneshotWrappedTokenTransferError::IncorrectGasFeeReceiver)]
    pub gas_fee_receiver: AccountInfo<'info>,

    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub from_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(mut, seeds = [BRIDGE_SEED], bump)]
    pub bridge: Account<'info, Bridge>,

    #[account(
        init,
        seeds = [OUTGOING_MESSAGE_SEED, bridge.nonce.to_le_bytes().as_ref()],
        bump,
        payer = payer,
        space = 8 + OutgoingMessage::oneshot_transfer_space(),
    )]
    pub outgoing_message: Account<'info, OutgoingMessage>,

    pub token_program: Program<'info, Token2022>,

    pub system_program: Program<'info, System>,
}

pub fn oneshot_wrapped_token_transfer_handler(
    ctx: Context<OneshotWrappedTokenTransfer>,
    gas_limit: u64,
    to: [u8; 20],
    amount: u64,
) -> Result<()> {
    let remote_token = process_wrapped_token_transfer_operation(
        &ctx.accounts.mint,
        &ctx.accounts.from_token_account,
        &ctx.accounts.from,
        &ctx.accounts.token_program,
        gas_limit,
        amount,
    )?;

    pay_for_gas(
        &ctx.accounts.system_program,
        &ctx.accounts.payer,
        &ctx.accounts.gas_fee_receiver,
        &mut ctx.accounts.bridge.eip1559,
        gas_limit,
    )?;

    *ctx.accounts.outgoing_message = OutgoingMessage::new_oneshot(
        ctx.accounts.from.key(),
        gas_limit,
        Operation::new_transfer(TransferOp {
            to,
            local_token: ctx.accounts.mint.key(),
            remote_token,
            amount,
        }),
    );
    ctx.accounts.bridge.nonce += 1;

    Ok(())
}

#[error_code]
pub enum OneshotWrappedTokenTransferError {
    #[msg("Incorrect gas fee receiver")]
    IncorrectGasFeeReceiver,
}
