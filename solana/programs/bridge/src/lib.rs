pub mod constants;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("Fb7KKBmjgKJh1N3aDUxLTj6TR3exH8Xi368bJ3AcDd5T");

#[program]
pub mod bridge {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Initializing: {:?}", ctx.program_id);
        Ok(())
    }

    pub fn bridge_sol_to(
        ctx: Context<BridgeSolTo>,
        remote_token: [u8; 20],
        to: [u8; 20],
        amount: u64,
        min_gas_limit: u32,
        extra_data: Vec<u8>,
    ) -> Result<()> {
        standard_bridge::bridge_sol_to_handler(
            ctx,
            remote_token,
            to,
            amount,
            min_gas_limit,
            extra_data,
        )
    }

    pub fn bridge_tokens_to(
        ctx: Context<BridgeTokensTo>,
        remote_token: [u8; 20],
        to: [u8; 20],
        amount: u64,
        min_gas_limit: u32,
        extra_data: Vec<u8>,
    ) -> Result<()> {
        standard_bridge::bridge_tokens_to_handler(
            ctx,
            remote_token,
            to,
            amount,
            min_gas_limit,
            extra_data,
        )
    }

    pub fn send_message(
        ctx: Context<SendMessage>,
        target: [u8; 20],
        message: Vec<u8>,
        min_gas_limit: u32,
    ) -> Result<()> {
        messenger::send_message_handler(ctx, target, message, min_gas_limit)
    }

    pub fn deposit_transaction(
        ctx: Context<DepositTransaction>,
        to: [u8; 20],
        gas_limit: u64,
        is_creation: bool,
        data: Vec<u8>,
    ) -> Result<()> {
        portal::deposit_transaction_handler(ctx, to, gas_limit, is_creation, data)
    }

    pub fn submit_root(ctx: Context<PostRoot>, root: [u8; 32], block_number: u64) -> Result<()> {
        post_root::submit_root_handler(ctx, root, block_number)
    }
}
