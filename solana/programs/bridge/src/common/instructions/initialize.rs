use anchor_lang::prelude::*;

use crate::common::{
    bridge::{Bridge, Eip1559},
    BRIDGE_SEED,
};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        seeds = [BRIDGE_SEED],
        bump,
        space = 8 + Bridge::INIT_SPACE
    )]
    pub bridge: Account<'info, Bridge>,

    pub system_program: Program<'info, System>,
}

pub fn initialize_handler(ctx: Context<Initialize>) -> Result<()> {
    let current_timestamp = Clock::get()?.unix_timestamp;

    *ctx.accounts.bridge = Bridge {
        base_block_number: 0,
        nonce: 0,
        eip1559: Eip1559::new(current_timestamp),
    };

    Ok(())
}
