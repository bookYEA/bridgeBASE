use anchor_lang::prelude::*;

use crate::{constants::MESSENGER_SEED, state::Messenger};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer, 
        seeds = [MESSENGER_SEED], 
        bump, 
        space = 8 + Messenger::INIT_SPACE
    )]
    pub messenger: Account<'info, Messenger>,

    pub system_program: Program<'info, System>,
}
