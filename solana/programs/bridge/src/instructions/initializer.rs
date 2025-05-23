use anchor_lang::prelude::*;

use crate::{Messenger, MESSENGER_SEED, VAULT_SEED, VERSION};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK: Vault PDA initialized with 0 space. For SOL, it receives SOL.
    /// For SPL, it's the authority for vault_token_account.
    /// The address is verified by the seeds constraint.
    #[account(
        init,
        payer = user,
        seeds = [VAULT_SEED, VERSION.to_le_bytes().as_ref()],
        space = 0,
        bump
    )]
    pub vault: AccountInfo<'info>,

    #[account(
        init, 
        payer = user, 
        seeds = [MESSENGER_SEED, VERSION.to_le_bytes().as_ref()], 
        bump, 
        space = 8 + Messenger::INIT_SPACE
    )]
    pub msg_state: Account<'info, Messenger>,

    pub system_program: Program<'info, System>,
}
