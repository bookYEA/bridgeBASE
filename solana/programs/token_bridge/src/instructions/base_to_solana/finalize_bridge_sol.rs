use anchor_lang::{
    prelude::*,
    system_program::{self, Transfer},
};

use portal::constants::PORTAL_AUTHORITY_SEED;

use crate::constants::{REMOTE_BRIDGE, SOL_VAULT_SEED};

#[derive(Accounts)]
#[instruction(remote_token: [u8; 20])]
pub struct FinalizeBridgeSol<'info> {
    /// CHECK: This is the Portal authority account.
    ///        It ensures that the call is triggered by the Portal program from an expected
    ///        remote sender (REMOTE_BRIDGE here).
    #[account(
        seeds = [PORTAL_AUTHORITY_SEED, REMOTE_BRIDGE.as_ref()],
        bump,
        seeds::program = portal::program::Portal::id()
    )]
    pub portal_authority: Signer<'info>,

    /// CHECK: This is the sol vault account for a specific remote token.
    #[account(mut, seeds = [SOL_VAULT_SEED, remote_token.as_ref()], bump)]
    pub sol_vault: AccountInfo<'info>,

    /// CHECK: This is the account to send the SOL to.
    #[account(mut)]
    pub to: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

pub fn finalize_bridge_sol_handler(
    ctx: Context<FinalizeBridgeSol>,
    remote_token: [u8; 20],
    amount: u64,
) -> Result<()> {
    let seeds: &[&[&[u8]]] = &[&[
        SOL_VAULT_SEED,
        remote_token.as_ref(),
        &[ctx.bumps.sol_vault],
    ]];

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.system_program.to_account_info(),
        Transfer {
            from: ctx.accounts.sol_vault.to_account_info(),
            to: ctx.accounts.to.to_account_info(),
        },
        seeds,
    );
    system_program::transfer(cpi_ctx, amount)
}
