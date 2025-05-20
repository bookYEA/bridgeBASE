use anchor_lang::prelude::*;

use crate::{Messenger, OutputRoot, MESSENGER_SEED, OUTPUT_ROOT_SEED, TRUSTED_ORACLE, VERSION};

#[derive(Accounts)]
#[instruction(root: [u8; 32], block_number: u64)]
pub struct PostRoot<'info> {
    #[account(
        init, 
        payer = payer, 
        space = 8 + OutputRoot::INIT_SPACE, 
        seeds = [OUTPUT_ROOT_SEED, &block_number.to_le_bytes()], 
        bump
    )]
    pub root: Account<'info, OutputRoot>,

    #[account(mut, seeds = [MESSENGER_SEED, VERSION.to_le_bytes().as_ref()], bump)]
    pub messenger: Account<'info, Messenger>,

    #[account(mut, address = TRUSTED_ORACLE)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn submit_root_handler(ctx: Context<PostRoot>, root: [u8; 32], block_number: u64) -> Result<()> {
    ctx.accounts.root.root = root;
    ctx.accounts.root.block_number = block_number;
    ctx.accounts.messenger.latest_block_number = block_number;
    Ok(())
}
