use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface, TransferChecked};

use portal::constants::PORTAL_AUTHORITY_SEED;

use crate::constants::{REMOTE_BRIDGE, TOKEN_VAULT_SEED};

#[derive(Accounts)]
#[instruction(remote_token: [u8; 20])]
pub struct BridgeBackSpl<'info> {
    /// CHECK: This is the Portal authority account.
    ///        It ensures that the call is triggered by the Portal program from an expected
    ///        remote sender (REMOTE_BRIDGE here).
    #[account(
        seeds = [PORTAL_AUTHORITY_SEED, REMOTE_BRIDGE.as_ref()],
        bump,
        seeds::program = portal::program::Portal::id()
    )]
    pub portal_authority: Signer<'info>,

    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        seeds = [TOKEN_VAULT_SEED, mint.key().as_ref(), remote_token.as_ref()],
        bump,
    )]
    pub token_vault: InterfaceAccount<'info, TokenAccount>,

    #[account(mut)]
    pub to_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn bridge_back_spl_handler(
    ctx: Context<BridgeBackSpl>,
    remote_token: [u8; 20],
    amount: u64,
) -> Result<()> {
    unlock_spl(&ctx, remote_token, amount)
}

fn unlock_spl(ctx: &Context<BridgeBackSpl>, remote_token: [u8; 20], amount: u64) -> Result<()> {
    let mint_key = ctx.accounts.mint.key();
    let seeds: &[&[&[u8]]] = &[&[
        TOKEN_VAULT_SEED,
        mint_key.as_ref(),
        remote_token.as_ref(),
        &[ctx.bumps.token_vault],
    ]];

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        TransferChecked {
            mint: ctx.accounts.mint.to_account_info(),
            from: ctx.accounts.token_vault.to_account_info(),
            to: ctx.accounts.to_token_account.to_account_info(),
            authority: ctx.accounts.token_vault.to_account_info(),
        },
        seeds,
    );
    token_interface::transfer_checked(cpi_ctx, amount, ctx.accounts.mint.decimals)
}
