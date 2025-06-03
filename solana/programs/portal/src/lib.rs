#![allow(unexpected_cfgs)]
use anchor_lang::prelude::*;

pub mod constants;
pub mod instructions;
pub mod internal;
pub mod solidity;
pub mod state;

use instructions::*;

declare_id!("4aRCwRtUjaoNA34AVLUmYVsyPRph2fNcAhXUxwHKUGtn");

#[program]
pub mod portal {
    use super::*;

    // Portal instructions

    pub fn initialize(_ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }

    pub fn send_call(
        ctx: Context<SendCall>,
        to: [u8; 20],
        gas_limit: u64,
        is_creation: bool,
        data: Vec<u8>,
    ) -> Result<()> {
        send_call_handler(ctx, to, gas_limit, is_creation, data)
    }

    pub fn register_output_root(
        ctx: Context<RegisterOutputRoot>,
        output_root: [u8; 32],
        _block_number: u64,
    ) -> Result<()> {
        register_output_root_handler(ctx, output_root)
    }

    pub fn prove_call(
        ctx: Context<ProveCall>,
        call_hash: [u8; 32],
        nonce: [u8; 32],
        sender: [u8; 20],
        data: Vec<u8>,
    ) -> Result<()> {
        prove_call_handler(ctx, call_hash, nonce, sender, data)
    }

    pub fn relay_call<'a, 'info>(
        ctx: Context<'a, '_, 'info, 'info, RelayCall<'info>>,
    ) -> Result<()> {
        relay_call_handler(ctx)
    }

    // Messenger instructions

    pub fn send_message(
        ctx: Context<SendMessage>,
        target: [u8; 20],
        message: Vec<u8>,
        min_gas_limit: u64,
    ) -> Result<()> {
        send_message_handler(ctx, target, message, min_gas_limit)
    }
}
