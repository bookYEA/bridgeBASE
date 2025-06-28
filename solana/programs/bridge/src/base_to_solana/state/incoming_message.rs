use anchor_lang::prelude::*;

#[account]
#[derive(Debug, Default)]
pub struct IncomingMessage {
    pub sender: [u8; 20],
    pub data: Vec<u8>,
    pub executed: bool,
}

impl IncomingMessage {
    pub fn space(data_len: usize) -> usize {
        20 + (4 + data_len) + 1
    }
}
