use anchor_lang::prelude::*;

use crate::{Messenger, MESSENGER_SEED};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init,
        payer = user, 
        seeds = [MESSENGER_SEED], 
        bump, 
        space = 8 + Messenger::INIT_SPACE
    )]
    pub messenger: Account<'info, Messenger>,

    pub system_program: Program<'info, System>,
}

pub fn initialize_handler(ctx: Context<Initialize>) -> Result<()> {
    let messenger = &mut ctx.accounts.messenger;   
    messenger.bump = ctx.bumps.messenger; 
    Ok(())
}