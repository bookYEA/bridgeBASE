use anchor_lang::prelude::*;

use crate::{
    common::{bridge::Bridge, BRIDGE_SEED},
    solana_to_base::{
        internal::pay_for_gas, OutgoingMessage, OutgoingMessageHeader, GAS_FEE_RECEIVER,
        MESSAGE_HEADER_SEED, OUTGOING_MESSAGE_SEED,
    },
};

#[derive(Accounts)]
#[instruction(id: u64)]
pub struct SendMessage<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub from: Signer<'info>,

    /// CHECK: This is the hardcoded gas fee receiver account.
    #[account(mut, address = GAS_FEE_RECEIVER @ SubmitMessageError::IncorrectGasFeeReceiver)]
    pub gas_fee_receiver: AccountInfo<'info>,

    #[account(mut, seeds = [BRIDGE_SEED], bump)]
    pub bridge: Account<'info, Bridge>,

    #[account(
        mut,
        close = payer,
        seeds = [MESSAGE_HEADER_SEED, from.key().as_ref(), id.to_le_bytes().as_ref()],
        bump,
    )]
    pub outgoing_message_header: Account<'info, OutgoingMessageHeader>,

    #[account(
        init,
        seeds = [
            OUTGOING_MESSAGE_SEED,
            bridge.nonce.to_le_bytes().as_ref(),
        ],
        bump,
        payer = payer,
        space = 8 + OutgoingMessage::composite_space(),
    )]
    pub outgoing_message: Account<'info, OutgoingMessage>,

    pub system_program: Program<'info, System>,
}

pub fn send_message_handler(ctx: Context<SendMessage>, _id: u64) -> Result<()> {
    pay_for_gas(
        &ctx.accounts.system_program,
        &ctx.accounts.payer,
        &ctx.accounts.gas_fee_receiver,
        &mut ctx.accounts.bridge.eip1559,
        ctx.accounts.outgoing_message_header.gas_limit,
    )?;

    *ctx.accounts.outgoing_message = OutgoingMessage::new_composite(
        ctx.accounts.from.key(),
        *ctx.accounts.outgoing_message_header,
    );

    ctx.accounts.bridge.nonce += 1;

    Ok(())
}

#[error_code]
pub enum SubmitMessageError {
    #[msg("Incorrect gas fee receiver")]
    IncorrectGasFeeReceiver,
}
