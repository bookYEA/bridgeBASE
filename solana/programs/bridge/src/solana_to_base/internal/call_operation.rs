use anchor_lang::prelude::*;

use crate::solana_to_base::{internal::min_gas_limit, Call, CallType, MAX_GAS_LIMIT_PER_MESSAGE};

pub fn process_call_operation(gas_limit: u64, call: &Call) -> Result<()> {
    require!(
        matches!(call.ty, CallType::Call | CallType::DelegateCall) || call.to == [0; 20],
        CallOperationError::CreationWithNonZeroTarget
    );

    require!(
        gas_limit >= min_gas_limit(call.data.len()),
        CallOperationError::GasLimitTooLow
    );

    require!(
        gas_limit <= MAX_GAS_LIMIT_PER_MESSAGE,
        CallOperationError::GasLimitExceeded
    );

    Ok(())
}

#[error_code]
pub enum CallOperationError {
    #[msg("Creation with non-zero target")]
    CreationWithNonZeroTarget,
    #[msg("Gas limit too low")]
    GasLimitTooLow,
    #[msg("Gas limit exceeded")]
    GasLimitExceeded,
}
