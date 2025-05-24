use anchor_lang::prelude::*;
use anchor_spl::{token_interface::Mint, token_interface::TokenInterface};

use crate::MINT_SEED;

#[derive(Accounts)]
#[instruction(remote_token: [u8; 20], decimals: u8)]
pub struct CreateMint<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        payer = signer,
        mint::decimals = decimals,
        mint::authority = crate::ID,
        mint::freeze_authority = crate::ID,
        seeds = [MINT_SEED, remote_token.as_ref(), decimals.to_le_bytes().as_ref()],
        bump
    )]
    pub mint: InterfaceAccount<'info, Mint>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn create_mint_handler(ctx: Context<CreateMint>) -> Result<()> {
    msg!("Created Mint Account: {:?}", ctx.accounts.mint.key());
    Ok(())
}
