use anchor_lang::prelude::*;

use crate::solana_to_base::{Operation, OutgoingMessageHeader};

#[derive(Debug, Clone, Eq, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub enum OutgoingMessageType {
    Composite(OutgoingMessageHeader),
    Oneshot {
        gas_limit: u64,
        operation: Operation,
    },
}

#[account]
#[derive(Debug, Eq, PartialEq)]
pub struct OutgoingMessage {
    pub from: Pubkey,
    pub message: OutgoingMessageType,
}

impl OutgoingMessage {
    pub fn new_composite(from: Pubkey, outgoing_message_header: OutgoingMessageHeader) -> Self {
        Self {
            from,
            message: OutgoingMessageType::Composite(outgoing_message_header),
        }
    }

    pub fn new_oneshot(from: Pubkey, gas_limit: u64, operation: Operation) -> Self {
        Self {
            from,
            message: OutgoingMessageType::Oneshot {
                gas_limit,
                operation,
            },
        }
    }

    pub fn composite_space() -> usize {
        // The space for a Oneshot message is always larger than the space for a Composite message.
        32 + (1 + Self::oneshot_transfer_space())
    }

    pub fn oneshot_call_space(data_len: usize) -> usize {
        32 + (1 + Operation::call_space(data_len))
    }

    pub fn oneshot_transfer_space() -> usize {
        32 + (1 + Operation::transfer_space())
    }
}
