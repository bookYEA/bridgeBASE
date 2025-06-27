use anchor_lang::prelude::*;

#[account]
#[derive(Debug, Copy, Eq, PartialEq, InitSpace)]
pub struct OutgoingMessageHeader {
    pub gas_limit: u64,
    pub operation_count: u16,
}
