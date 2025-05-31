use anchor_lang::prelude::*;

use crate::{Messenger, MESSENGER_SEED, AUTHORITY_VAULT_SEED, VERSION};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK: This is the vault authority PDA.
    ///        - For SOL, it receives SOL.
    ///        - For SPL, it's the authority of the vault token account.
    #[account(
        init,
        payer = user,
        seeds = [AUTHORITY_VAULT_SEED, VERSION.to_le_bytes().as_ref()],
        bump,
        space = 0,
    )]
    pub authority_vault: AccountInfo<'info>,

    #[account(
        init, 
        payer = user, 
        seeds = [MESSENGER_SEED, VERSION.to_le_bytes().as_ref()], 
        bump, 
        space = 8 + Messenger::INIT_SPACE
    )]
    pub messenger: Account<'info, Messenger>,

    pub system_program: Program<'info, System>,
}
