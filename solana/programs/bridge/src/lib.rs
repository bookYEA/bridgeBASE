pub mod constants;
pub mod instructions;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;

declare_id!("Gwi8c92gteE63Z9i78nXmStWWP9tf6wLN5jaXC9tdGjp");

#[program]
pub mod bridge {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Initializing: {:?}", ctx.program_id);
        Ok(())
    }

    pub fn deposit_transaction(
        ctx: Context<DepositTransaction>,
        to: [u8; 20],
        value: u64,
        gas_limit: u64,
        is_creation: bool,
        data: Vec<u8>,
    ) -> Result<()> {
        portal::deposit_transaction_handler(ctx, to, value, gas_limit, is_creation, data)
    }

    pub fn send_message(
        ctx: Context<SendMessage>,
        target: [u8; 20],
        message: Vec<u8>,
        value: u64,
        min_gas_limit: u32,
    ) -> Result<()> {
        messenger::send_message_handler(ctx, target, message, value, min_gas_limit)
    }
}
