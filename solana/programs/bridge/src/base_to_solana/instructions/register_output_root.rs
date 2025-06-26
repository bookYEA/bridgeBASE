use anchor_lang::prelude::*;

use crate::{
    base_to_solana::{
        constants::{OUTPUT_ROOT_SEED, TRUSTED_ORACLE},
        state::OutputRoot,
    },
    common::{constants::BRIDGE_SEED, state::bridge::Bridge},
};

#[derive(Accounts)]
#[instruction(_output_root: [u8; 32], block_number: u64)]
pub struct RegisterOutputRoot<'info> {
    #[account(mut, address = TRUSTED_ORACLE @ RegisterOutputRootError::Unauthorized)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + OutputRoot::INIT_SPACE,
        seeds = [OUTPUT_ROOT_SEED, &block_number.to_le_bytes()],
        bump
    )]
    pub root: Account<'info, OutputRoot>,

    #[account(
        mut,
        seeds = [BRIDGE_SEED],
        bump,
    )]
    pub bridge: Account<'info, Bridge>,

    pub system_program: Program<'info, System>,
}

pub fn register_output_root_handler(
    ctx: Context<RegisterOutputRoot>,
    output_root: [u8; 32],
    block_number: u64,
) -> Result<()> {
    require!(
        block_number - ctx.accounts.bridge.base_block_number % 300 == 0,
        RegisterOutputRootError::InvalidBlockNumber
    );

    // TODO: Plug some ISM verification here.

    ctx.accounts.root.root = output_root;
    ctx.accounts.bridge.base_block_number = block_number;

    Ok(())
}

#[error_code]
pub enum RegisterOutputRootError {
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("InvalidBlockNumber")]
    InvalidBlockNumber,
}
