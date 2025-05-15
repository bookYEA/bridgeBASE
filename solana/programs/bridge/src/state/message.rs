use anchor_lang::prelude::*;

#[derive(InitSpace)]
#[account]
pub struct Message {
    pub is_valid: bool,
}
