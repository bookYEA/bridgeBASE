use anchor_lang::prelude::*;

pub const MESSAGE_HEADER_SEED: &[u8] = b"message_header";
pub const OPERATION_SEED: &[u8] = b"operation";
pub const OUTGOING_MESSAGE_SEED: &[u8] = b"outgoing_message";
pub const NATIVE_SOL_PUBKEY: Pubkey = pubkey!("SoL1111111111111111111111111111111111111111");
pub const MAX_GAS_LIMIT_PER_MESSAGE: u64 = 100_000_000;
pub const GAS_COST_SCALER_DP: u64 = 10u64.pow(6);
pub const GAS_COST_SCALER: u64 = 1_000_000;

pub const RELAY_MESSAGES_CALL_ABI_ENCODING_OVERHEAD: usize = 544;
pub const RELAY_MESSAGES_TRANSFER_ABI_ENCODING_OVERHEAD: usize = 480;
pub const RELAY_MESSAGES_TRANSFER_AND_CALL_ABI_ENCODING_OVERHEAD: usize = 704;

mod private {
    use anchor_lang::prelude::*;

    #[cfg(feature = "devnet")]
    pub mod config {
        use super::*;

        pub const GAS_FEE_RECEIVER: Pubkey = pubkey!("eEwCrQLBdQchykrkYitkYUZskd7MPrU2YxBXcPDPnMt");
    }

    #[cfg(feature = "mainnet")]
    pub mod config {
        use super::*;

        pub const GAS_FEE_RECEIVER: Pubkey = pubkey!("11111111111111111111111111111111");
    }

    #[cfg(not(any(feature = "devnet", feature = "mainnet")))]
    pub mod config {
        use super::*;

        pub const GAS_FEE_RECEIVER: Pubkey =
            pubkey!("CB8GXDdZDSD5uqfeow1qfp48ouayxXGpw7ycmoovuQMX");
    }
}

pub use private::config::*;

#[cfg(test)]
mod tests {
    use alloy_sol_types::{SolInterface, SolValue};

    use crate::solana_to_base::internal::solidity::{
        Bridge::{self, relayMessagesCall},
        Call, CallType, IncomingMessage, MessageType, Transfer,
    };

    use super::*;

    #[test]
    fn test_relay_messages_call_abi_encoding_overhead() {
        let call = Call {
            ty: CallType::Call,
            to: [0; 20].into(),
            value: 0,
            data: "".into(),
        };

        let incoming_msg = IncomingMessage {
            nonce: 0,
            sender: [0; 32].into(),
            gasLimit: 0,
            ty: MessageType::Call,
            data: call.abi_encode().into(),
        };

        let call = Bridge::BridgeCalls::relayMessages(relayMessagesCall {
            messages: vec![incoming_msg],
            ismData: "".as_bytes().into(),
        });

        assert_eq!(
            call.abi_encoded_size(),
            RELAY_MESSAGES_CALL_ABI_ENCODING_OVERHEAD
        );
    }

    #[test]
    fn test_relay_messages_transfer_abi_encoding_overhead() {
        let transfer = Transfer {
            localToken: [0; 20].into(),
            remoteToken: [0; 32].into(),
            to: [0; 32].into(),
            remoteAmount: 0,
        };

        let incoming_msg = IncomingMessage {
            nonce: 0,
            sender: [0; 32].into(),
            gasLimit: 0,
            ty: MessageType::Transfer,
            data: transfer.abi_encode().into(),
        };

        let call = Bridge::BridgeCalls::relayMessages(relayMessagesCall {
            messages: vec![incoming_msg],
            ismData: "".as_bytes().into(),
        });

        assert_eq!(
            call.abi_encoded_size(),
            RELAY_MESSAGES_TRANSFER_ABI_ENCODING_OVERHEAD
        );
    }

    #[test]
    fn test_relay_messages_transfer_and_call_abi_encoding_overhead() {
        let transfer = Transfer {
            localToken: [0; 20].into(),
            remoteToken: [0; 32].into(),
            to: [0; 32].into(),
            remoteAmount: 0,
        };

        let call = Call {
            ty: CallType::Call,
            to: [0; 20].into(),
            value: 0,
            data: "".into(),
        };

        let incoming_msg = IncomingMessage {
            nonce: 0,
            sender: [0; 32].into(),
            gasLimit: 0,
            ty: MessageType::TransferAndCall,
            data: (transfer, call).abi_encode().into(),
        };

        let call = Bridge::BridgeCalls::relayMessages(relayMessagesCall {
            messages: vec![incoming_msg],
            ismData: "".as_bytes().into(),
        });

        assert_eq!(
            call.abi_encoded_size(),
            RELAY_MESSAGES_TRANSFER_AND_CALL_ABI_ENCODING_OVERHEAD
        );
    }
}
