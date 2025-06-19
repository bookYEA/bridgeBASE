pub mod send_call;

pub use send_call::*;

use anchor_lang::prelude::*;

#[derive(Debug, Copy, Clone, AnchorSerialize, AnchorDeserialize)]
#[repr(u8)]
pub enum CallType {
    Call,
    DelegateCall,
    Create,
    Create2,
}

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct Call {
    pub ty: CallType,
    pub to: [u8; 20],
    pub gas_limit: u64,
    pub data: Vec<u8>,
}
