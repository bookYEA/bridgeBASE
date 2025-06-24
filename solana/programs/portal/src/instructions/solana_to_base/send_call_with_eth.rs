use anchor_lang::prelude::*;
use anchor_spl::token_2022::BurnChecked;
use anchor_spl::token_interface::{self, Mint, Token2022, TokenAccount};
use common::metadata::PartialTokenMetadata;

use crate::constants::{
    GAS_FEE_RECEIVER, NATIVE_ETH_TOKEN, PORTAL_SEED, TOKEN_BRIDGE, WRAPPED_TOKEN_SEED,
};
use crate::internal::send_call_internal;
use crate::state::Portal;

use super::{Call, CallType};

#[derive(Accounts)]
pub struct SendCallWithEth<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub authority: Signer<'info>,

    /// CHECK: This is the hardcoded gas fee receiver account.
    #[account(mut, address = GAS_FEE_RECEIVER)]
    pub gas_fee_receiver: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [PORTAL_SEED],
        bump,
    )]
    pub portal: Account<'info, Portal>,

    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub from_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Program<'info, Token2022>,

    pub system_program: Program<'info, System>,
}

pub fn send_call_with_eth_handler(
    ctx: Context<SendCallWithEth>,
    ty: CallType,
    to: [u8; 20],
    gas_limit: u64,
    value: u64,
    data: Vec<u8>,
) -> Result<()> {
    let partial_token_metadata =
        PartialTokenMetadata::try_from(&ctx.accounts.mint.to_account_info())?;

    require!(
        partial_token_metadata.remote_token == NATIVE_ETH_TOKEN,
        SendCallWithEthError::NotNativeEthToken
    );

    // Ensure that the given mint account is a legit wrapped ETH SPL token of the `token_bridge` program.
    let (wrapped_token, _) = Pubkey::find_program_address(
        &[
            WRAPPED_TOKEN_SEED,
            ctx.accounts.mint.decimals.to_le_bytes().as_ref(),
            partial_token_metadata.hash().as_ref(),
        ],
        &TOKEN_BRIDGE,
    );

    require_keys_neq!(
        wrapped_token,
        ctx.accounts.mint.key(),
        SendCallWithEthError::IncorrectMint
    );

    // Scaled the ETH value according to the mint's scaler exponent.
    // NOTE: Very unlikely that an ETH value will overflow a u128.
    let scaler = 10u128.pow(partial_token_metadata.scaler_exponent as u32);
    let remote_value = (value as u128) * scaler;

    burn(&ctx, value)?;

    send_call_internal(
        &ctx.accounts.system_program,
        &ctx.accounts.payer,
        &ctx.accounts.gas_fee_receiver,
        &mut ctx.accounts.portal,
        ctx.accounts.authority.key(),
        Call {
            ty,
            to,
            gas_limit,
            remote_value,
            data,
        },
    )
}

fn burn(ctx: &Context<SendCallWithEth>, amount: u64) -> Result<()> {
    let cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        BurnChecked {
            mint: ctx.accounts.mint.to_account_info(),
            from: ctx.accounts.from_token_account.to_account_info(),
            authority: ctx.accounts.payer.to_account_info(),
        },
    );
    token_interface::burn_checked(cpi_ctx, amount, ctx.accounts.mint.decimals)
}

#[error_code]
pub enum SendCallWithEthError {
    #[msg("Not native ETH token")]
    NotNativeEthToken,
    #[msg("Incorrect mint")]
    IncorrectMint,
    #[msg("Overflow")]
    Overflow,
}
