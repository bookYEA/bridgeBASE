#![allow(unexpected_cfgs)]
use anchor_lang::prelude::*;

pub mod constants;
pub mod instructions;
pub mod internal;
pub mod solidity;

use instructions::*;

declare_id!("3R8PyojdmUTwB6FAkzjwRZsfAzucA9E1nK4ydNARvT8b");

#[program]
pub mod token_bridge {
    use super::*;

    // Solana to Base

    pub fn bridge_sol(
        ctx: Context<BridgeSol>,
        remote_token: [u8; 20],
        to: [u8; 20],
        amount: u64,
        min_gas_limit: u64,
        extra_data: Vec<u8>,
    ) -> Result<()> {
        bridge_sol_handler(ctx, remote_token, to, amount, min_gas_limit, extra_data)
    }

    pub fn bridge_spl(
        ctx: Context<BridgeSpl>,
        remote_token: [u8; 20],
        to: [u8; 20],
        amount: u64,
        min_gas_limit: u64,
        extra_data: Vec<u8>,
    ) -> Result<()> {
        bridge_spl_handler(ctx, remote_token, to, amount, min_gas_limit, extra_data)
    }

    pub fn bridge_back_token(
        ctx: Context<BridgeBackToken>,
        remote_token: [u8; 20],
        _remote_decimals: u8, // NOTE: Only used to compute the PDA seed of the Mint.
        to: [u8; 20],
        amount: u64,
        min_gas_limit: u64,
        extra_data: Vec<u8>,
    ) -> Result<()> {
        bridge_back_token_handler(ctx, remote_token, to, amount, min_gas_limit, extra_data)
    }

    // Base to Solana

    pub fn wrap_token(
        ctx: Context<WrapToken>,
        _remote_token: [u8; 20], // NOTE: Only used to compute the PDA seed of the Mint.
        remote_decimals: u8,     // NOTE: Only used to compute the PDA seed of the Mint.
    ) -> Result<()> {
        wrap_token_handler(ctx, remote_decimals)
    }

    pub fn finalize_bridge_sol(
        ctx: Context<FinalizeBridgeSol>,
        remote_token: [u8; 20],
        amount: u64,
    ) -> Result<()> {
        finalize_bridge_sol_handler(ctx, remote_token, amount)
    }

    pub fn finalize_bridge_spl(
        ctx: Context<FinalizeBridgeSpl>,
        remote_token: [u8; 20],
        amount: u64,
    ) -> Result<()> {
        finalize_bridge_spl_handler(ctx, remote_token, amount)
    }

    pub fn finalize_bridge_token(
        ctx: Context<FinalizeBridgeToken>,
        _expected_mint: Pubkey, // NOTE: Only used to assert the mint PDA pubkey.
        remote_token: [u8; 20],
        amount: u64,
    ) -> Result<()> {
        finalize_bridge_token_handler(ctx, remote_token, amount)
    }
}
