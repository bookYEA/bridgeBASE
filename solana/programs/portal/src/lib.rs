#![allow(unexpected_cfgs)]
use anchor_lang::prelude::*;

#[cfg(test)]
mod test_utils;

pub mod constants;
pub mod instructions;
pub mod internal;
pub mod state;

use instructions::*;
use internal::Proof;

declare_id!("4jduFi9ShXq258vmY4GroJUYTRQnd9GWZxzK8zTxTmmw");

#[program]
pub mod portal {

    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        initialize_handler(ctx)
    }

    pub fn send_call(
        ctx: Context<SendCall>,
        ty: CallType,
        to: [u8; 20],
        gas_limit: u64,
        data: Vec<u8>,
    ) -> Result<()> {
        send_call_handler(ctx, ty, to, gas_limit, data)
    }

    pub fn send_call_with_eth(
        ctx: Context<SendCallWithEth>,
        ty: CallType,
        to: [u8; 20],
        gas_limit: u64,
        value: u64,
        data: Vec<u8>,
    ) -> Result<()> {
        send_call_with_eth_handler(ctx, ty, to, gas_limit, value, data)
    }

    pub fn register_output_root(
        ctx: Context<RegisterOutputRoot>,
        output_root: [u8; 32],
        block_number: u64,
    ) -> Result<()> {
        register_output_root_handler(ctx, output_root, block_number)
    }

    pub fn prove_call(
        ctx: Context<ProveCall>,
        nonce: [u8; 32],
        sender: [u8; 20],
        data: Vec<u8>,
        proof: Proof,
    ) -> Result<()> {
        prove_call_handler(ctx, nonce, sender, data, proof)
    }

    pub fn relay_call<'a, 'info>(
        ctx: Context<'a, '_, 'info, 'info, RelayCall<'info>>,
    ) -> Result<()> {
        relay_call_handler(ctx)
    }
}
