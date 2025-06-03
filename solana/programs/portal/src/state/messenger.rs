use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Messenger {
    pub nonce: u64,
}
