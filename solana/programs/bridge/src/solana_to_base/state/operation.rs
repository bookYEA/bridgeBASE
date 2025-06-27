use std::cmp::max;

use anchor_lang::prelude::*;

#[derive(Debug, Clone, Eq, PartialEq, AnchorSerialize, AnchorDeserialize, InitSpace)]
pub struct Transfer {
    pub to: [u8; 20],
    pub local_token: Pubkey,
    pub remote_token: [u8; 20],
    pub amount: u64,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, AnchorSerialize, AnchorDeserialize, InitSpace)]
pub enum CallType {
    Call,
    DelegateCall,
    Create,
    Create2,
}

#[derive(Debug, Clone, Eq, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub struct Call {
    pub ty: CallType,
    pub to: [u8; 20],
    pub value: u128,
    pub data: Vec<u8>,
}

impl Call {
    pub fn space(data_len: usize) -> usize {
        CallType::INIT_SPACE + 20 + 16 + (4 + data_len)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub enum TransferOrCall {
    Transfer(Transfer),
    Call(Call),
}

#[account]
#[derive(Debug, Eq, PartialEq)]
pub struct Operation(pub TransferOrCall);

impl Operation {
    pub fn new_call(call: Call) -> Self {
        Self(TransferOrCall::Call(call))
    }

    pub fn new_transfer(transfer: Transfer) -> Self {
        Self(TransferOrCall::Transfer(transfer))
    }

    pub fn call_space(data_len: usize) -> usize {
        let call_space = Call::space(data_len);
        1 + max(Transfer::INIT_SPACE, call_space)
    }

    pub fn transfer_space() -> usize {
        // The space for a Transfer is always larger than the space for a Call with no data.
        1 + Transfer::INIT_SPACE
    }
}
