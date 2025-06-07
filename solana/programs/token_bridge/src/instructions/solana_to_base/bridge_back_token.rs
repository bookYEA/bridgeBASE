use alloy_primitives::{FixedBytes, U256};
use alloy_sol_types::SolCall;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, BurnChecked, Mint, Token2022, TokenAccount};

use portal::{cpi as portal_cpi, program::Portal};

use crate::{
    constants::BRIDGE_AUTHORITY_SEED, instructions::PartialTokenMetadata,
    internal::cpi_send_message, solidity::Bridge,
};

#[derive(Accounts)]
#[instruction(remote_token: [u8; 20], remote_decimals: u8)]
pub struct BridgeBackToken<'info> {
    // Bridge accounts
    #[account(mut)]
    pub from: Signer<'info>,

    /// CHECK: This is the Bridge authority account.
    ///        It is used as the authority when CPIing to the Portal program.
    #[account(mut, seeds = [BRIDGE_AUTHORITY_SEED], bump)]
    pub bridge_authority: AccountInfo<'info>,

    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub from_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Program<'info, Token2022>,

    pub portal: Program<'info, Portal>,

    // Portal accounts
    // TODO: use composite accounts once figured out how to make them compile.
    /// CHECK: Going to be checked in the cpi.
    pub gas_fee_receiver: AccountInfo<'info>,

    /// CHECK: Going to be checked in the cpi.
    pub messenger: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

pub fn bridge_back_token_handler(
    ctx: Context<BridgeBackToken>,
    to: [u8; 20],
    amount: u64,
    min_gas_limit: u64,
    extra_data: Vec<u8>,
) -> Result<()> {
    let partial_token_metadata =
        PartialTokenMetadata::try_from(&ctx.accounts.mint.to_account_info())?;

    burn(&ctx, amount)?;

    cpi_send_message(
        &ctx.accounts.portal,
        portal_cpi::accounts::SendMessage {
            payer: ctx.accounts.from.to_account_info(),
            authority: ctx.accounts.bridge_authority.to_account_info(),
            gas_fee_receiver: ctx.accounts.gas_fee_receiver.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            messenger: ctx.accounts.messenger.to_account_info(),
        },
        ctx.bumps.bridge_authority,
        Bridge::finalizeBridgeTokenCall {
            localToken: partial_token_metadata.remote_token.into(), // NOTE: Intentional flip the token so that when executing on Base it's correct.
            remoteToken: FixedBytes::from(ctx.accounts.mint.key().to_bytes()), // NOTE: Intentional flip the token so that when executing on Base it's correct.
            from: FixedBytes::from(ctx.accounts.from.key().to_bytes()),
            to: to.into(),
            amount: U256::from(amount),
            extraData: extra_data.into(),
        }
        .abi_encode(),
        min_gas_limit,
    )
}

fn burn(ctx: &Context<BridgeBackToken>, amount: u64) -> Result<()> {
    let cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        BurnChecked {
            mint: ctx.accounts.mint.to_account_info(),
            from: ctx.accounts.from_token_account.to_account_info(),
            authority: ctx.accounts.from.to_account_info(),
        },
    );
    token_interface::burn_checked(cpi_ctx, amount, ctx.accounts.mint.decimals)
}

#[error_code]
pub enum BridgeBackTokenError {
    #[msg("Incorrect mint account")]
    IncorrectMintAccount,
}
