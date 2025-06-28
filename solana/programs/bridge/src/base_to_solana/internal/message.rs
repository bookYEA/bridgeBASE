use anchor_lang::prelude::{
    borsh::{BorshDeserialize, BorshSerialize},
    *,
};

use crate::base_to_solana::{
    token::{FinalizeBridgeSol, FinalizeBridgeSpl, FinalizeBridgeWrappedToken},
    Ix,
};

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub enum Message {
    Call(Vec<Ix>),
    Transfer { transfer: Transfer, ixs: Vec<Ix> },
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub enum Transfer {
    Sol(FinalizeBridgeSol),
    Spl(FinalizeBridgeSpl),
    WrappedToken(FinalizeBridgeWrappedToken),
}
