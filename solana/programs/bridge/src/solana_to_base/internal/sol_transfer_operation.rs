use anchor_lang::{
    prelude::*,
    system_program::{self, Transfer},
};

use crate::solana_to_base::{min_gas_limit, MAX_GAS_LIMIT_PER_MESSAGE};

pub fn process_sol_transfer_operation<'info>(
    sol_vault: AccountInfo<'info>,
    from: AccountInfo<'info>,
    system_program: &Program<'info, System>,
    gas_limit: u64,
    amount: u64,
) -> Result<()> {
    require!(
        gas_limit >= min_gas_limit(0),
        SolTransferOperationError::GasLimitTooLow
    );

    require!(
        gas_limit <= MAX_GAS_LIMIT_PER_MESSAGE,
        SolTransferOperationError::GasLimitExceeded
    );

    // Lock the sol from the user into the SOL vault.
    let cpi_ctx = CpiContext::new(
        system_program.to_account_info(),
        Transfer {
            from,
            to: sol_vault,
        },
    );

    system_program::transfer(cpi_ctx, amount)?;

    Ok(())
}

#[error_code]
pub enum SolTransferOperationError {
    #[msg("Gas limit too low")]
    GasLimitTooLow,
    #[msg("Gas limit exceeded")]
    GasLimitExceeded,
}
