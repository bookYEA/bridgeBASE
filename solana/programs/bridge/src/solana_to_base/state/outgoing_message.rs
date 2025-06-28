use anchor_lang::prelude::*;

#[derive(Debug, Clone, Eq, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub struct Transfer {
    pub to: [u8; 20],
    pub local_token: Pubkey,
    pub remote_token: [u8; 20],
    pub amount: u64,
    pub call: Option<Call>,
}

impl Transfer {
    pub fn space(data_len: Option<usize>) -> usize {
        20 + 32 + 20 + 8 + 1 + Call::space(data_len.unwrap_or_default())
    }
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
pub enum Message {
    Call(Call),
    Transfer(Transfer),
}

#[account]
#[derive(Debug, Eq, PartialEq)]
pub struct OutgoingMessage {
    pub sender: Pubkey,
    pub gas_limit: u64,
    pub message: Message,
}

impl OutgoingMessage {
    pub fn new_call(sender: Pubkey, gas_limit: u64, call: Call) -> Self {
        Self {
            sender,
            gas_limit,
            message: Message::Call(call),
        }
    }

    pub fn new_transfer(sender: Pubkey, gas_limit: u64, transfer: Transfer) -> Self {
        Self {
            sender,
            gas_limit,
            message: Message::Transfer(transfer),
        }
    }

    pub fn space(data_len: Option<usize>) -> usize {
        // The transfer variant is always bigger than the call variant (as it embeds an optional call)
        32 + 8 + (1 + Transfer::space(data_len))
    }
}
