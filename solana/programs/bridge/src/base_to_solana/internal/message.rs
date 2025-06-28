use anchor_lang::prelude::*;

use crate::base_to_solana::Ix;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub enum Message {
    Call(Vec<Ix>),
    Transfer { transfer: Ix, ixs: Vec<Ix> },
}
