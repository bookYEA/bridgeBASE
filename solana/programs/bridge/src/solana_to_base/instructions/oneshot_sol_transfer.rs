use anchor_lang::prelude::*;

use crate::{
    common::{bridge::Bridge, BRIDGE_SEED, SOL_VAULT_SEED},
    solana_to_base::{
        pay_for_gas, process_sol_transfer_operation, Operation, OutgoingMessage,
        Transfer as TransferOp, GAS_FEE_RECEIVER, NATIVE_SOL_PUBKEY, OUTGOING_MESSAGE_SEED,
    },
};

#[derive(Accounts)]
#[instruction(_gas_limit: u64, _to: [u8; 20], remote_token: [u8; 20], _amount: u64)]
pub struct OneshotSolTransfer<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub from: Signer<'info>,

    /// CHECK: This is the hardcoded gas fee receiver account.
    #[account(mut, address = GAS_FEE_RECEIVER @ OneshotSolTransferError::IncorrectGasFeeReceiver)]
    pub gas_fee_receiver: AccountInfo<'info>,

    /// CHECK: This is the SOL vault account.
    #[account(
        mut,
        seeds = [SOL_VAULT_SEED, remote_token.as_ref()],
        bump,
    )]
    pub sol_vault: AccountInfo<'info>,

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

    pub system_program: Program<'info, System>,
}

pub fn oneshot_sol_transfer_handler(
    ctx: Context<OneshotSolTransfer>,
    gas_limit: u64,
    to: [u8; 20],
    remote_token: [u8; 20],
    amount: u64,
) -> Result<()> {
    process_sol_transfer_operation(
        ctx.accounts.sol_vault.to_account_info(),
        ctx.accounts.from.to_account_info(),
        &ctx.accounts.system_program,
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
            local_token: NATIVE_SOL_PUBKEY,
            remote_token,
            amount,
        }),
    );
    ctx.accounts.bridge.nonce += 1;

    Ok(())
}

#[error_code]
pub enum OneshotSolTransferError {
    #[msg("Incorrect gas fee receiver")]
    IncorrectGasFeeReceiver,
    #[msg("Gas limit too low")]
    GasLimitTooLow,
    #[msg("Gas limit exceeded")]
    GasLimitExceeded,
}
