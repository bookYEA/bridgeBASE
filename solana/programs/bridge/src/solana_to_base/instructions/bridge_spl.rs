use anchor_lang::prelude::*;
use anchor_spl::token_interface::{transfer_checked, TransferChecked};
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::common::PartialTokenMetadata;
use crate::solana_to_base::{check_and_pay_for_gas, check_call};
use crate::{
    common::{bridge::Bridge, BRIDGE_SEED, TOKEN_VAULT_SEED},
    solana_to_base::{
        Call, OutgoingMessage, Transfer as TransferOp, GAS_FEE_RECEIVER, OUTGOING_MESSAGE_SEED,
    },
};

#[derive(Accounts)]
#[instruction(_gas_limit: u64, _to: [u8; 20], remote_token: [u8; 20], _amount: u64, call: Option<Call>)]
pub struct BridgeSpl<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub from: Signer<'info>,

    /// CHECK: This is the hardcoded gas fee receiver account.
    #[account(mut, address = GAS_FEE_RECEIVER @ BridgeSplError::IncorrectGasFeeReceiver)]
    pub gas_fee_receiver: AccountInfo<'info>,

    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub from_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(mut, seeds = [TOKEN_VAULT_SEED, remote_token.as_ref()], bump)]
    pub token_vault: InterfaceAccount<'info, TokenAccount>,

    #[account(mut, seeds = [BRIDGE_SEED], bump)]
    pub bridge: Account<'info, Bridge>,

    #[account(
        init,
        seeds = [OUTGOING_MESSAGE_SEED, bridge.nonce.to_le_bytes().as_ref()],
        bump,
        payer = payer,
        space = 8 + OutgoingMessage::space(call.map(|c| c.data.len())),
    )]
    pub outgoing_message: Account<'info, OutgoingMessage>,

    pub token_program: Interface<'info, TokenInterface>,

    pub system_program: Program<'info, System>,
}

pub fn bridge_spl_handler(
    ctx: Context<BridgeSpl>,
    gas_limit: u64,
    to: [u8; 20],
    remote_token: [u8; 20],
    amount: u64,
    call: Option<Call>,
) -> Result<()> {
    if let Some(call) = &call {
        check_call(call)?;
    }

    let message = OutgoingMessage::new_transfer(
        ctx.accounts.from.key(),
        gas_limit,
        TransferOp {
            to,
            local_token: ctx.accounts.mint.key(),
            remote_token,
            amount,
            call,
        },
    );

    check_and_pay_for_gas(
        &ctx.accounts.system_program,
        &ctx.accounts.payer,
        &ctx.accounts.gas_fee_receiver,
        &mut ctx.accounts.bridge.eip1559,
        gas_limit,
        message.relay_messages_tx_size(),
    )?;

    // Check that the provided mint is not a wrapped token.
    // Wrapped tokens should be handled by the wrapped_token_transfer_operation branch which burns the token from the user.
    require!(
        PartialTokenMetadata::try_from(&ctx.accounts.mint.to_account_info()).is_err(),
        BridgeSplError::MintIsWrappedToken
    );

    // Lock the token from the user into the token vault.
    let cpi_accounts = TransferChecked {
        mint: ctx.accounts.mint.to_account_info(),
        from: ctx.accounts.from_token_account.to_account_info(),
        to: ctx.accounts.token_vault.to_account_info(),
        authority: ctx.accounts.from.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);

    transfer_checked(cpi_ctx, amount, ctx.accounts.mint.decimals)?;

    *ctx.accounts.outgoing_message = message;
    ctx.accounts.bridge.nonce += 1;

    Ok(())
}

#[error_code]
pub enum BridgeSplError {
    #[msg("Incorrect gas fee receiver")]
    IncorrectGasFeeReceiver,
    #[msg("Mint is a wrapped token")]
    MintIsWrappedToken,
}
