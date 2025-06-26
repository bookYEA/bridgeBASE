use anchor_lang::prelude::*;

#[derive(Debug, Copy, Clone, Eq, PartialEq, AnchorSerialize, AnchorDeserialize, InitSpace)]
#[repr(u8)]
pub enum CallType {
    Call,
    DelegateCall,
    Create,
    Create2,
}

#[account]
#[derive(Debug, InitSpace)]
pub struct Call {
    pub nonce: u64,
    pub ty: CallType,
    pub from: Pubkey,
    pub to: [u8; 20],
    pub gas_limit: u64,
    pub remote_value: u128,
    #[max_len(1080)]
    pub data: Vec<u8>,
}
