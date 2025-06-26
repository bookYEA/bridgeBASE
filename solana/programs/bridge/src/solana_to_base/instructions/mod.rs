use anchor_lang::prelude::*;

use crate::{
    common::bridge::Eip1559,
    solana_to_base::{GAS_COST_SCALER, GAS_COST_SCALER_DP},
};

pub mod send_call_message;
pub mod send_transfer_message;

pub use send_call_message::*;
pub use send_transfer_message::*;

fn pay_for_gas<'info>(
    system_program: &Program<'info, System>,
    payer: &Signer<'info>,
    gas_fee_receiver: &AccountInfo<'info>,
    eip1559: &mut Eip1559,
    gas_limit: u64,
) -> Result<()> {
    // Get the base fee for the current window
    let current_timestamp = Clock::get()?.unix_timestamp;
    let base_fee = eip1559.refresh_base_fee(current_timestamp);

    // Record gas usage for this transaction
    eip1559.add_gas_usage(gas_limit);

    let gas_cost = gas_limit * base_fee * GAS_COST_SCALER / GAS_COST_SCALER_DP;

    let cpi_ctx = CpiContext::new(
        system_program.to_account_info(),
        anchor_lang::system_program::Transfer {
            from: payer.to_account_info(),
            to: gas_fee_receiver.clone(),
        },
    );
    anchor_lang::system_program::transfer(cpi_ctx, gas_cost)?;

    Ok(())
}

fn min_gas_limit(total_data_len: usize) -> u64 {
    const RELAY_CALL_GAS_BUFFER: u64 = 40_000;
    const RELAY_CALL_OVERHEAD_GAS: u64 = 40_000;

    total_data_len as u64 * 40 + 21_000 + RELAY_CALL_GAS_BUFFER + RELAY_CALL_OVERHEAD_GAS
}
