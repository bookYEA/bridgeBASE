use anchor_lang::prelude::*;

#[derive(InitSpace)]
#[account]
pub struct Messenger {
    pub msg_nonce: u64,
    pub latest_block_number: u64,
}
