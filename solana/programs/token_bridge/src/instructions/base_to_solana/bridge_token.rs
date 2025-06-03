use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface, MintToChecked};

use portal::constants::PORTAL_AUTHORITY_SEED;

use crate::constants::{REMOTE_BRIDGE, WRAPPED_TOKEN_SEED};

#[derive(Accounts)]
#[instruction(remote_token: [u8; 20])]
pub struct BridgeToken<'info> {
    /// CHECK: This is the Portal authority account.
    ///        It ensures that the call is triggered by the Portal program from an expected
    ///        remote sender (REMOTE_BRIDGE here).
    #[account(
        seeds = [PORTAL_AUTHORITY_SEED, REMOTE_BRIDGE.as_ref()],
        bump,
        seeds::program = portal::program::Portal::id()
    )]
    pub portal_authority: Signer<'info>,

    #[account(
        mut,
        seeds = [
            WRAPPED_TOKEN_SEED, 
            remote_token.as_ref(),
            mint.decimals.to_le_bytes().as_ref()
        ],
        bump
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub to_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn bridge_token_handler(
    ctx: Context<BridgeToken>,
    remote_token: [u8; 20],
    amount: u64,
) -> Result<()> {
    mint(&ctx, &remote_token, amount)
}


fn mint(ctx: &Context<BridgeToken>, remote_token: &[u8; 20], amount: u64) -> Result<()> {
    let decimals = ctx.accounts.mint.decimals.to_le_bytes();
    let seeds: &[&[&[u8]]] = &[&[WRAPPED_TOKEN_SEED, remote_token, &decimals, &[ctx.bumps.mint]]];

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        MintToChecked {
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.to_token_account.to_account_info(),
            authority: ctx.accounts.mint.to_account_info(),
        },
        seeds,
    );
    token_interface::mint_to_checked(cpi_ctx, amount, ctx.accounts.mint.decimals)
}