use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct IncomingMessage {
    pub sender: [u8; 20],
    #[max_len(1080)]
    pub data: Vec<u8>,
    pub executed: bool,
}
