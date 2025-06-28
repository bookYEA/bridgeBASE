use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct OutputRoot {
    pub root: [u8; 32],
}
