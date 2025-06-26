#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;

pub mod base_to_solana;
pub mod common;
pub mod solana_to_base;

use base_to_solana::*;
use common::*;
use solana_to_base::*;

declare_id!("EF3xsxZGWWJX9T7vCPb7hEgyJQKEj1mgSNLMNvF8a7cj");

#[program]
pub mod bridge {
    use super::*;

    // Common

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        initialize_handler(ctx)
    }

    // Base -> Solana

    pub fn register_output_root(
        ctx: Context<RegisterOutputRoot>,
        output_root: [u8; 32],
        block_number: u64,
    ) -> Result<()> {
        register_output_root_handler(ctx, output_root, block_number)
    }

    pub fn prove_message(
        ctx: Context<ProveMessage>,
        nonce: u64,
        sender: [u8; 20],
        data: Vec<u8>,
        proof: Proof,
    ) -> Result<()> {
        prove_message_handler(ctx, nonce, sender, data, proof)
    }

    pub fn relay_message<'a, 'info>(
        ctx: Context<'a, '_, 'info, 'info, RelayMessage<'info>>,
    ) -> Result<()> {
        relay_message_handler(ctx)
    }

    // Solana -> Base

    pub fn send_call_message(
        ctx: Context<SendCallMessage>,
        call_type: CallType,
        to: [u8; 20],
        gas_limit: u64,
        value: u128,
        data: Vec<u8>,
    ) -> Result<()> {
        send_call_message_handler(ctx, call_type, to, gas_limit, value, data)
    }

    pub fn send_transfer_message(
        ctx: Context<SendTransferMessage>,
        to: [u8; 20],
        gas_limit: u64,
        local_token: Pubkey,
        remote_token: [u8; 20],
        local_amount: u64,
    ) -> Result<()> {
        send_transfer_message_handler(ctx, to, gas_limit, local_token, remote_token, local_amount)
    }
}
