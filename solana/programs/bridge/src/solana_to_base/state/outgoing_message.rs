use anchor_lang::prelude::*;

#[derive(Debug, Copy, Clone, Eq, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub enum CallType {
    Call,
    DelegateCall,
    Create,
    Create2,
}

#[derive(Debug, Clone, Eq, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub enum OutgoingMessagePayload {
    Transfer {
        to: [u8; 20],
        local_token: Pubkey,
        remote_token: [u8; 20],
        local_amount: u64,
    },
    Call {
        call_type: CallType,
        to: [u8; 20],
        value: u128,
        data: Vec<u8>,
    },
}

#[account]
#[derive(Debug)]
pub struct OutgoingMessage {
    pub nonce: u64,
    pub sender: Pubkey,
    pub gas_limit: u64,
    pub payload: OutgoingMessagePayload,
}

impl OutgoingMessage {
    pub fn space(data_len: Option<usize>) -> usize {
        let payload_space = match data_len {
            // Transfer
            None => 32 + 20 + 20 + 8,
            // Call
            Some(data_len) => (1 + 1) + 20 + 8 + 16 + (4 + data_len),
        };

        8 + 32 + 8 + 1 + payload_space
    }
}
