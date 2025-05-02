pub mod constants;
pub mod instructions;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;

declare_id!("Gwi8c92gteE63Z9i78nXmStWWP9tf6wLN5jaXC9tdGjp");

#[program]
pub mod bridge {
    use super::*;

    pub fn deposit_transaction(
        ctx: Context<DepositTransaction>,
        to: [u8; 20],
        value: u64,
        gas_limit: u64,
        is_creation: bool,
        data: Vec<u8>,
    ) -> Result<()> {
        deposit_transaction::deposit_transaction_handler(
            ctx,
            to,
            value,
            gas_limit,
            is_creation,
            data,
        )
    }
}
