use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, MintToChecked, Token2022, TokenAccount};

use portal::constants::PORTAL_AUTHORITY_SEED;

use crate::{
    constants::{REMOTE_BRIDGE, WRAPPED_TOKEN_SEED},
    instructions::PartialTokenMetadata,
};

#[derive(Accounts)]
pub struct FinalizeBridgeToken<'info> {
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
        // NOTE: We check that the PDA derivation is correct in the handler to optimize the CPI.
        // seeds = [
        //     WRAPPED_TOKEN_SEED,
        //     mint.decimals.to_le_bytes().as_ref(),
        //     PartialTokenMetadata::try_from(&mint.to_account_info())?.hash().as_ref()
        // ],
        // bump,
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub to_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Program<'info, Token2022>,
}

pub fn finalize_bridge_token_handler(ctx: Context<FinalizeBridgeToken>, amount: u64) -> Result<()> {
    let decimals_bytes = ctx.accounts.mint.decimals.to_le_bytes();
    let metadata_hash =
        PartialTokenMetadata::try_from(&ctx.accounts.mint.to_account_info())?.hash();

    let seeds: &[&[u8]] = &[
        WRAPPED_TOKEN_SEED,
        decimals_bytes.as_ref(),
        metadata_hash.as_ref(),
    ];

    let (expected_mint, expected_bump) = Pubkey::find_program_address(seeds, ctx.program_id);
    require_keys_eq!(
        ctx.accounts.mint.to_account_info().key(),
        expected_mint,
        FinalizeBridgeTokenError::IncorrectMintAccount
    );

    let seeds: &[&[&[u8]]] = &[&[
        WRAPPED_TOKEN_SEED,
        decimals_bytes.as_ref(),
        metadata_hash.as_ref(),
        &[expected_bump],
    ]];

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

#[error_code]
pub enum FinalizeBridgeTokenError {
    #[msg("Incorrect mint account")]
    IncorrectMintAccount,
}
