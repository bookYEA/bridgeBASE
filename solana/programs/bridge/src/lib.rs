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

    pub fn create_call_operation(
        ctx: Context<CreateCallOperation>,
        id: u64,
        call_type: CallType,
        gas_limit: u64,
        to: [u8; 20],
        value: u128,
        data: Vec<u8>,
    ) -> Result<()> {
        create_call_operation_handler(ctx, id, call_type, gas_limit, to, value, data)
    }

    pub fn oneshot_call(ctx: Context<OneshotCall>, gas_limit: u64, call: Call) -> Result<()> {
        oneshot_call_handler(ctx, gas_limit, call)
    }

    pub fn create_sol_transfer_operation(
        ctx: Context<CreateSolTransferOperation>,
        id: u64,
        gas_limit: u64,
        to: [u8; 20],
        remote_token: [u8; 20],
        amount: u64,
    ) -> Result<()> {
        create_sol_transfer_operation_handler(ctx, id, gas_limit, to, remote_token, amount)
    }

    pub fn oneshot_sol_transfer(
        ctx: Context<OneshotSolTransfer>,
        gas_limit: u64,
        to: [u8; 20],
        remote_token: [u8; 20],
        amount: u64,
    ) -> Result<()> {
        oneshot_sol_transfer_handler(ctx, gas_limit, to, remote_token, amount)
    }

    pub fn create_spl_transfer_operation(
        ctx: Context<CreateSplTransferOperation>,
        id: u64,
        gas_limit: u64,
        to: [u8; 20],
        remote_token: [u8; 20],
        amount: u64,
    ) -> Result<()> {
        create_spl_transfer_operation_handler(ctx, id, gas_limit, to, remote_token, amount)
    }

    pub fn oneshot_spl_transfer(
        ctx: Context<OneshotSplTransfer>,
        gas_limit: u64,
        to: [u8; 20],
        remote_token: [u8; 20],
        amount: u64,
    ) -> Result<()> {
        oneshot_spl_transfer_handler(ctx, gas_limit, to, remote_token, amount)
    }

    pub fn create_wrapped_token_transfer_operation(
        ctx: Context<CreateWrappedTokenTransferOperation>,
        id: u64,
        gas_limit: u64,
        to: [u8; 20],
        amount: u64,
    ) -> Result<()> {
        create_wrapped_token_transfer_operation_handler(ctx, id, gas_limit, to, amount)
    }

    pub fn oneshot_wrapped_token_transfer(
        ctx: Context<OneshotWrappedTokenTransfer>,
        gas_limit: u64,
        to: [u8; 20],
        amount: u64,
    ) -> Result<()> {
        oneshot_wrapped_token_transfer_handler(ctx, gas_limit, to, amount)
    }

    pub fn send_message(ctx: Context<SendMessage>, id: u64) -> Result<()> {
        send_message_handler(ctx, id)
    }
}
