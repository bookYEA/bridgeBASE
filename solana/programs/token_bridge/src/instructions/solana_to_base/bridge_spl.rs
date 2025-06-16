use alloy_primitives::{FixedBytes, U256};
use alloy_sol_types::SolCall;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface, TransferChecked};

use portal::{cpi as portal_cpi, program::Portal};

use crate::{
    constants::{BRIDGE_AUTHORITY_SEED, TOKEN_VAULT_SEED},
    internal::{cpi_send_message, is_wrapped_token},
    solidity::Bridge,
};

#[derive(Accounts)]
#[instruction(remote_token: [u8; 20])]
pub struct BridgeSpl<'info> {
    // Bridge accounts
    #[account(mut)]
    pub from: Signer<'info>,

    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub from_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init_if_needed,
        seeds = [TOKEN_VAULT_SEED, mint.key().as_ref(), remote_token.as_ref()],
        bump,
        payer = from,
        token::mint = mint,
        token::authority = token_vault
    )]
    pub token_vault: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,

    pub portal: Program<'info, Portal>,

    // Portal remaining accounts
    /// CHECK: Checked by the Portal program that we CPI into.
    #[account(mut)]
    pub messenger: AccountInfo<'info>,

    /// CHECK: This is the Bridge authority account.
    ///        It is used as the authority when CPIing to the Portal program.
    #[account(seeds = [BRIDGE_AUTHORITY_SEED], bump)]
    pub bridge_authority: AccountInfo<'info>,

    /// CHECK: Checked by the Portal program that we CPI into.
    #[account(mut)]
    pub gas_fee_receiver: AccountInfo<'info>,

    /// CHECK: Checked by the Portal program that we CPI into.
    #[account(mut)]
    pub eip1559: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

pub fn bridge_spl_handler(
    ctx: Context<BridgeSpl>,
    remote_token: [u8; 20],
    to: [u8; 20],
    amount: u64,
    min_gas_limit: u64,
    extra_data: Vec<u8>,
) -> Result<()> {
    // Check that the provided mint is not a wrapped `remote_token`, in which case the `bridge_back_token` instruction should be called instead.
    require!(
        !is_wrapped_token(ctx.program_id, &ctx.accounts.mint, &remote_token).0,
        BridgeSplError::MintIsAWrappedToken
    );

    lock_spl(&ctx, amount)?;

    cpi_send_message(
        &ctx.accounts.portal,
        portal_cpi::accounts::SendMessage {
            payer: ctx.accounts.from.to_account_info(),
            authority: ctx.accounts.bridge_authority.to_account_info(),
            gas_fee_receiver: ctx.accounts.gas_fee_receiver.to_account_info(),
            eip1559: ctx.accounts.eip1559.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            messenger: ctx.accounts.messenger.to_account_info(),
        },
        ctx.bumps.bridge_authority,
        Bridge::finalizeBridgeTokenCall {
            localToken: remote_token.into(), // NOTE: Intentionally flip the tokens so that when executing on Base it's correct.
            remoteToken: FixedBytes::from(ctx.accounts.mint.key().to_bytes()), // NOTE: Intentionally flip the tokens so that when executing on Base it's correct.
            from: FixedBytes::from(ctx.accounts.from.key().to_bytes()),
            to: to.into(),
            amount: U256::from(amount),
            extraData: extra_data.into(),
        }
        .abi_encode(),
        min_gas_limit,
    )
}

fn lock_spl(ctx: &Context<BridgeSpl>, amount: u64) -> Result<()> {
    let cpi_accounts = TransferChecked {
        mint: ctx.accounts.mint.to_account_info(),
        from: ctx.accounts.from_token_account.to_account_info(),
        to: ctx.accounts.token_vault.to_account_info(),
        authority: ctx.accounts.from.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
    token_interface::transfer_checked(cpi_ctx, amount, ctx.accounts.mint.decimals)
}

#[error_code]
pub enum BridgeSplError {
    #[msg("Mint is a wrapped token")]
    MintIsAWrappedToken,
}
