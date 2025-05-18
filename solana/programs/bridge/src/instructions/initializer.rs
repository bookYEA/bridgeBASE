use anchor_lang::prelude::*;

use crate::{Vault, MESSENGER_SEED, VAULT_SEED};

use super::Messenger;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK: This is the vault PDA. We are only using it to transfer SOL via CPI
    /// to the system program, so no data checks are required. The address is
    /// verified by the seeds constraint.
    #[account(
        init,
        payer = user,
        seeds = [VAULT_SEED],
        space = 8 + Vault::INIT_SPACE,
        bump
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        init, 
        payer = user, 
        seeds = [MESSENGER_SEED], 
        bump, 
        space = 8 + Messenger::INIT_SPACE
    )]
    pub msg_state: Account<'info, Messenger>,

    pub system_program: Program<'info, System>,
}
