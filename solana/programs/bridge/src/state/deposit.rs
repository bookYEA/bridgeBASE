use anchor_lang::prelude::*;

#[derive(InitSpace)]
#[account]
pub struct Deposit {
    pub balance: u64,
}
