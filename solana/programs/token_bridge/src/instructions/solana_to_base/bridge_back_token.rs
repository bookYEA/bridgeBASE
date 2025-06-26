use alloy_primitives::FixedBytes;
use alloy_sol_types::SolCall;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, BurnChecked, Mint, Token2022, TokenAccount};

use common::metadata::PartialTokenMetadata;
use portal::{constants::NATIVE_ETH_TOKEN, cpi as portal_cpi, program::Portal};

use crate::{
    constants::{BRIDGE_AUTHORITY_SEED, WRAPPED_TOKEN_SEED},
    internal::{cpi_send_call, cpi_send_call_with_eth},
    solidity::Bridge,
};

#[derive(Accounts)]
pub struct BridgeBackToken<'info> {
    // Bridge accounts
    #[account(mut)]
    pub from: Signer<'info>,

    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub from_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Program<'info, Token2022>,

    pub portal_program: Program<'info, Portal>,

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
    pub portal: AccountInfo<'info>,

    /// CHECK: Checked by the Portal program that we CPI into.
    #[account(mut)]
    pub call: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

pub fn bridge_back_token_handler(
    ctx: Context<BridgeBackToken>,
    to: [u8; 20],
    amount: u64,
    gas_limit: u64,
    extra_data: Vec<u8>,
) -> Result<()> {
    let partial_token_metadata =
        PartialTokenMetadata::try_from(&ctx.accounts.mint.to_account_info())?;

    let finalize_bridge_token_call = Bridge::finalizeBridgeTokenCall {
        localToken: partial_token_metadata.remote_token.into(), // NOTE: Intentional flip the token so that when executing on Base it's correct.
        remoteToken: FixedBytes::from(ctx.accounts.mint.key().to_bytes()), // NOTE: Intentional flip the token so that when executing on Base it's correct.
        from: FixedBytes::from(ctx.accounts.from.key().to_bytes()),
        to: to.into(),
        remoteAmount: amount,
        extraData: extra_data.into(),
    }
    .abi_encode();

    if partial_token_metadata.remote_token == NATIVE_ETH_TOKEN {
        // NOTE: We don't need to check the mint account is the expected PDA here because the Portal program already ensures that.
        //       for the native ETH remote token on Base.

        // NOTE: Wrapped ETH burning is handled by the Portal program.

        cpi_send_call_with_eth(
            &ctx.accounts.portal_program,
            portal_cpi::accounts::SendCallWithEth {
                payer: ctx.accounts.from.to_account_info(),
                authority: ctx.accounts.bridge_authority.to_account_info(),
                gas_fee_receiver: ctx.accounts.gas_fee_receiver.to_account_info(),
                portal: ctx.accounts.portal.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                from_token_account: ctx.accounts.from_token_account.to_account_info(),
                token_program: ctx.accounts.token_program.to_account_info(),
                call: ctx.accounts.call.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
            },
            ctx.bumps.bridge_authority,
            gas_limit,
            amount,
            finalize_bridge_token_call,
        )
    } else {
        // NOTE: This check validates that the provided mint is a valid wrapped token SPL created by the token_bridge program.
        //       While not strictly necessary (since the TokenBridge on Base side also validates the token pair), it provides
        //       an early safety check to prevent users from accidentally burning incorrect tokens and losing their funds.
        let (wrapped_token, _) = Pubkey::find_program_address(
            &[
                WRAPPED_TOKEN_SEED,
                ctx.accounts.mint.decimals.to_le_bytes().as_ref(),
                partial_token_metadata.hash().as_ref(),
            ],
            ctx.program_id,
        );

        require_keys_eq!(
            wrapped_token,
            ctx.accounts.mint.key(),
            BridgeBackTokenError::MintIsNotWrappedToken
        );

        burn(&ctx, amount)?;

        cpi_send_call(
            &ctx.accounts.portal_program,
            portal_cpi::accounts::SendCall {
                payer: ctx.accounts.from.to_account_info(),
                authority: ctx.accounts.bridge_authority.to_account_info(),
                gas_fee_receiver: ctx.accounts.gas_fee_receiver.to_account_info(),
                portal: ctx.accounts.portal.to_account_info(),
                call: ctx.accounts.call.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
            },
            ctx.bumps.bridge_authority,
            gas_limit,
            finalize_bridge_token_call,
        )
    }
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
    #[msg("Mint is not a wrapped token")]
    MintIsNotWrappedToken,
}
