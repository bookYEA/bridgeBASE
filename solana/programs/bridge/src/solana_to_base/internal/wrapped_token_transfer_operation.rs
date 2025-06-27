use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::Token2022,
    token_interface::{self, BurnChecked, Mint, TokenAccount},
};

use crate::{
    common::PartialTokenMetadata,
    solana_to_base::{internal::min_gas_limit, MAX_GAS_LIMIT_PER_MESSAGE},
};

pub fn process_wrapped_token_transfer_operation<'info>(
    mint: &InterfaceAccount<'info, Mint>,
    from_token_account: &InterfaceAccount<'info, TokenAccount>,
    from: &Signer<'info>,
    token_program: &Program<'info, Token2022>,
    gas_limit: u64,
    amount: u64,
) -> Result<[u8; 20]> {
    require!(
        gas_limit >= min_gas_limit(0),
        WrappedTokenTransferOperationError::GasLimitTooLow
    );

    require!(
        gas_limit <= MAX_GAS_LIMIT_PER_MESSAGE,
        WrappedTokenTransferOperationError::GasLimitExceeded
    );

    // Get the token metadata from the mint.
    let partial_token_metadata = PartialTokenMetadata::try_from(&mint.to_account_info())?;

    // Burn the token from the user.
    let cpi_ctx = CpiContext::new(
        token_program.to_account_info(),
        BurnChecked {
            mint: mint.to_account_info(),
            from: from_token_account.to_account_info(),
            authority: from.to_account_info(),
        },
    );

    token_interface::burn_checked(cpi_ctx, amount, mint.decimals)?;

    Ok(partial_token_metadata.remote_token)
}

#[error_code]
pub enum WrappedTokenTransferOperationError {
    #[msg("Gas limit too low")]
    GasLimitTooLow,
    #[msg("Gas limit exceeded")]
    GasLimitExceeded,
}
