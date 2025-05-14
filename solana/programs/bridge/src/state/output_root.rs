use anchor_lang::prelude::*;

#[derive(InitSpace)]
#[account]
pub struct OutputRoot {
    pub root: [u8; 32],
    pub block_number: u64,
}
