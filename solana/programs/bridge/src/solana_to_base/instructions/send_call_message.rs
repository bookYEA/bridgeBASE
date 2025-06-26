use anchor_lang::prelude::*;

use crate::{
    common::{bridge::Bridge, BRIDGE_SEED},
    solana_to_base::{
        instructions::{min_gas_limit, pay_for_gas},
        CallType, OutgoingMessage, OutgoingMessagePayload, GAS_FEE_RECEIVER, OUTGOING_MESSAGE_SEED,
    },
};

#[derive(Accounts)]
#[instruction(call_type: CallType, to: [u8; 20], gas_limit: u64, value: u128, data: Vec<u8>)]
pub struct SendCallMessage<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub authority: Signer<'info>,

    /// CHECK: This is the hardcoded gas fee receiver account.
    #[account(mut, address = GAS_FEE_RECEIVER @ SendCallMessageError::IncorrectGasFeeReceiver)]
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
        space = 8 + OutgoingMessage::space(Some(data.len())),
    )]
    pub message: Account<'info, OutgoingMessage>,

    pub system_program: Program<'info, System>,
}

pub fn send_call_message_handler(
    ctx: Context<SendCallMessage>,
    call_type: CallType,
    to: [u8; 20],
    gas_limit: u64,
    value: u128,
    data: Vec<u8>,
) -> Result<()> {
    require!(
        matches!(call_type, CallType::Call | CallType::DelegateCall) || to == [0; 20],
        SendCallMessageError::CreationWithNonZeroTarget
    );

    require!(
        gas_limit >= min_gas_limit(data.len()),
        SendCallMessageError::GasLimitTooLow
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
        payload: OutgoingMessagePayload::Call {
            call_type,
            to,
            value,
            data,
        },
    };

    *ctx.accounts.message = message;

    ctx.accounts.bridge.nonce += 1;

    Ok(())
}

#[error_code]
pub enum SendCallMessageError {
    #[msg("Incorrect gas fee receiver")]
    IncorrectGasFeeReceiver,
    #[msg("Creation with non-zero target")]
    CreationWithNonZeroTarget,
    #[msg("Gas limit too low")]
    GasLimitTooLow,
}
