use anchor_lang::prelude::*;

use crate::{
    common::{bridge::Bridge, BRIDGE_SEED},
    solana_to_base::{
        instructions::{min_gas_limit, pay_for_gas},
        OutgoingMessage, OutgoingMessagePayload, GAS_FEE_RECEIVER, OUTGOING_MESSAGE_SEED,
    },
};

#[derive(Accounts)]
pub struct SendTransferMessage<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub authority: Signer<'info>,

    /// CHECK: This is the hardcoded gas fee receiver account.
    #[account(mut, address = GAS_FEE_RECEIVER @ SendTransferMessageError::IncorrectGasFeeReceiver)]
    pub gas_fee_receiver: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [BRIDGE_SEED],
        bump,
    )]
    pub bridge: Account<'info, Bridge>,

    #[account(
        init,
        seeds = [OUTGOING_MESSAGE_SEED, bridge.nonce.to_le_bytes().as_ref()],
        bump,
        payer = payer,
        space = 8 + OutgoingMessage::space(None),
    )]
    pub message: Account<'info, OutgoingMessage>,

    pub system_program: Program<'info, System>,
}

pub fn send_transfer_message_handler(
    ctx: Context<SendTransferMessage>,
    to: [u8; 20],
    gas_limit: u64,
    local_token: Pubkey,
    remote_token: [u8; 20],
    local_amount: u64,
) -> Result<()> {
    require!(
        gas_limit >= min_gas_limit(0),
        SendTransferMessageError::GasLimitTooLow
    );

    pay_for_gas(
        &ctx.accounts.system_program,
        &ctx.accounts.payer,
        &ctx.accounts.gas_fee_receiver,
        &mut ctx.accounts.bridge.eip1559,
        gas_limit,
    )?;

    let message = OutgoingMessage {
        nonce: ctx.accounts.bridge.nonce,
        sender: ctx.accounts.authority.key(),
        gas_limit,
        payload: OutgoingMessagePayload::Transfer {
            to,
            local_token,
            remote_token,
            local_amount,
        },
    };

    *ctx.accounts.message = message;

    ctx.accounts.bridge.nonce += 1;

    Ok(())
}

#[error_code]
pub enum SendTransferMessageError {
    #[msg("Incorrect gas fee receiver")]
    IncorrectGasFeeReceiver,
    #[msg("Gas limit too low")]
    GasLimitTooLow,
}
