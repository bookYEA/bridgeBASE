#![allow(unexpected_cfgs)]
use anchor_lang::prelude::*;

pub mod constants;
pub mod instructions;
pub mod internal;
pub mod solidity;

#[cfg(test)]
pub mod test_utils;

use instructions::*;

use crate::internal::metadata::PartialTokenMetadata;

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
        to: [u8; 20],
        amount: u64,
        min_gas_limit: u64,
        extra_data: Vec<u8>,
    ) -> Result<()> {
        bridge_back_token_handler(ctx, to, amount, min_gas_limit, extra_data)
    }

    // Base to Solana

    pub fn wrap_token(
        ctx: Context<WrapToken>,
        decimals: u8,
        partial_token_metadata: PartialTokenMetadata,
        min_gas_limit: u64,
    ) -> Result<()> {
        wrap_token_handler(ctx, decimals, partial_token_metadata, min_gas_limit)
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

    pub fn finalize_bridge_token(ctx: Context<FinalizeBridgeToken>, amount: u64) -> Result<()> {
        finalize_bridge_token_handler(ctx, amount)
    }
}
