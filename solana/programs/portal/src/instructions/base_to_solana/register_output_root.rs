use anchor_lang::prelude::*;

use crate::{
    constants::{MESSENGER_SEED, OUTPUT_ROOT_SEED, TRUSTED_ORACLE},
    state::{Messenger, OutputRoot},
};

#[derive(Accounts)]
#[instruction(_output_root: [u8; 32], block_number: u64)]
pub struct RegisterOutputRoot<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + OutputRoot::INIT_SPACE,
        seeds = [OUTPUT_ROOT_SEED, &block_number.to_le_bytes()],
        bump
    )]
    pub root: Account<'info, OutputRoot>,

    #[account(mut, seeds = [MESSENGER_SEED], bump)]
    pub messenger: Account<'info, Messenger>,

    #[account(mut, address = TRUSTED_ORACLE @ RegisterOutputRootError::Unauthorized)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn register_output_root_handler(
    ctx: Context<RegisterOutputRoot>,
    output_root: [u8; 32],
) -> Result<()> {
    ctx.accounts.root.root = output_root;

    Ok(())
}

#[error_code]
pub enum RegisterOutputRootError {
    #[msg("Unauthorized")]
    Unauthorized,
}
