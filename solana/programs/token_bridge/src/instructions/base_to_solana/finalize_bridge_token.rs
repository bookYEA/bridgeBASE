use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface, MintToChecked};

use portal::constants::PORTAL_AUTHORITY_SEED;

use crate::constants::{REMOTE_BRIDGE, WRAPPED_TOKEN_SEED};

#[derive(Accounts)]
#[instruction(expected_mint: Pubkey, remote_token: [u8; 20])]
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
        seeds = [
            WRAPPED_TOKEN_SEED, 
            remote_token.as_ref(),
            mint.decimals.to_le_bytes().as_ref()
        ],
        bump,

        // IMPORTANT: We ensure that the `expected_mint` that was provided by the user on the Base side matches
        //            the mint PDA that was recomputed here and only then mint the tokens.
        //
        //            Without this check, a user could bridge a token from Base to Solana and register a deposit 
        //            for [ERC20][SPL_A] but actually receive a SPL_B on Solana. After that the user would be
        //            able to bridge the SPL_B back to Base and receive some ERC20, that were locked by a different
        //            user, creating an imbalance in the ERC20 <-> SPL_B route.
        //
        //            * Example of flow that would be possible without the check:
        //
        //            1. User calls `bridgeToken` on Base with `localToken` being ERC20 and `remoteToken` being
        //               SPL_A, and thus locks some amount into the [ERC20][SPL_A] mapping entry.
        //
        //            2. This call is executed on Solana but the mint PDA, seeded on `localToken` and `decimals`,
        //               derived does not give the SPL_A mint, but rather the SPL_B mint.
        //
        //            3. The Solana bridge mints SPL_B to the user.
        //
        //            4. User initiates a bridge back to Base which burns the SPL_B from the user and calls the 
        //               Base's `finalizeBridgeToken` method with `localToken` being SPL_B and `remoteToken` being 
        //               ERC20.
        //
        //            5. The Base bridge will try to unlock the amount from the [SPL_B][ERC20] mapping entry,
        //               creating an imbalance in the SPL_B <-> ERC20 route.
        //
        //            * Why checking that the mint PDA matches the expected mint solves this?
        //
        //            By checking that the mint PDA matches the expected mint, we ensure that we do NOT mint the right
        //            SPL_B token if the user locked its deposit on [ERC20][SPL_A] instead of [ERC20_B][SPL_B] on the
        //            Base side. Because the SPL_B token are never minted on Solana, the step 4 would never happen.
        //
        //            While this guarantees that the bridge routes stay balanced, it does not prevent users from
        //            permanently locking their funds on the Base side if they provided an invalid `remoteToken`
        //            on the `bridgeToken` call.
        address = expected_mint @ FinalizeBridgeTokenError::IncorrectMint
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub to_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn finalize_bridge_token_handler(
    ctx: Context<FinalizeBridgeToken>,
    remote_token: [u8; 20],
    amount: u64,
) -> Result<()> {    
    mint(&ctx, &remote_token, amount)
}


fn mint(ctx: &Context<FinalizeBridgeToken>, remote_token: &[u8; 20], amount: u64) -> Result<()> {
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

#[error_code]
pub enum FinalizeBridgeTokenError {
    #[msg("Incorrect mint")]
    IncorrectMint,
}