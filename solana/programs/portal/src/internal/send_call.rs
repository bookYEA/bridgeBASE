use anchor_lang::prelude::*;

use crate::{
    constants::{GAS_COST_SCALER, GAS_COST_SCALER_DP},
    instructions::{solana_to_base::CallSent, Call, CallType},
    state::{Eip1559, Portal},
};

pub fn send_call_internal<'info>(
    system_program: &Program<'info, System>,
    payer: &Signer<'info>,
    gas_fee_receiver: &AccountInfo<'info>,
    portal: &mut Account<'info, Portal>,
    from: Pubkey,
    call: Call,
) -> Result<()> {
    // Can't hurt to check this here (though it's not strictly preventing the user to footgun themselves)
    require!(
        matches!(call.ty, CallType::Call | CallType::DelegateCall) || call.to == [0; 20],
        SendCallError::CreationWithNonZeroTarget
    );

    require!(
        call.gas_limit >= min_gas_limit(call.data.len()),
        SendCallError::GasLimitTooLow
    );

    pay_for_gas(
        system_program,
        payer,
        gas_fee_receiver,
        &mut portal.eip1559,
        call.gas_limit,
    )?;

    emit!(CallSent {
        nonce: portal.nonce,
        from,
        call,
    });

    portal.nonce += 1;

    Ok(())
}

fn min_gas_limit(total_data_len: usize) -> u64 {
    const RELAY_CALL_GAS_BUFFER: u64 = 40_000;
    const RELAY_CALL_OVERHEAD_GAS: u64 = 40_000;

    total_data_len as u64 * 40 + 21_000 + RELAY_CALL_GAS_BUFFER + RELAY_CALL_OVERHEAD_GAS
}

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

#[error_code]
pub enum SendCallError {
    #[msg("Incorrect gas fee receiver")]
    IncorrectGasFeeReceiver,
    #[msg("Creation with non-zero target")]
    CreationWithNonZeroTarget,
    #[msg("Gas limit too low")]
    GasLimitTooLow,
}
