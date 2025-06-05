use anchor_lang::prelude::*;
use anchor_spl::{token_interface::Mint, token_interface::TokenInterface};

use crate::constants::WRAPPED_TOKEN_SEED;

#[derive(Accounts)]
#[instruction(remote_token: [u8; 20], remote_decimals: u8)]
pub struct WrapToken<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        mint::decimals = remote_decimals,
        mint::authority = mint,
        mint::freeze_authority = mint,
        seeds = [
            WRAPPED_TOKEN_SEED,
            remote_token.as_ref(),
            remote_decimals.to_le_bytes().as_ref(),
        ],
        bump
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

pub fn wrap_token_handler(_ctx: Context<WrapToken>, remote_decimals: u8) -> Result<()> {
    require!(remote_decimals <= 9, WrapTokenError::InvalidDecimals);
    Ok(())
}

#[error_code]
pub enum WrapTokenError {
    #[msg("Invalid decimals")]
    InvalidDecimals,
}
