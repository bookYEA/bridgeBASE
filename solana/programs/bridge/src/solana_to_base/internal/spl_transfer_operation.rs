use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface, TransferChecked};

use crate::{
    common::PartialTokenMetadata,
    solana_to_base::{internal::min_gas_limit, MAX_GAS_LIMIT_PER_MESSAGE},
};

pub fn process_spl_transfer_operation<'info>(
    mint: &InterfaceAccount<'info, Mint>,
    from_token_account: &InterfaceAccount<'info, TokenAccount>,
    token_vault: &InterfaceAccount<'info, TokenAccount>,
    from: &Signer<'info>,
    token_program: &Interface<'info, TokenInterface>,
    gas_limit: u64,
    amount: u64,
) -> Result<()> {
    require!(
        gas_limit >= min_gas_limit(0),
        SplTransferOperationError::GasLimitTooLow
    );

    require!(
        gas_limit <= MAX_GAS_LIMIT_PER_MESSAGE,
        SplTransferOperationError::GasLimitExceeded
    );

    // Check that the provided mint is not a wrapped token.
    // Wrapped tokens should be handled by the wrapped_token_transfer_operation branch which burns the token from the user.
    require!(
        PartialTokenMetadata::try_from(&mint.to_account_info()).is_err(),
        SplTransferOperationError::MintIsAWrappedToken
    );

    // Lock the token from the user into the token vault.
    let cpi_accounts = TransferChecked {
        mint: mint.to_account_info(),
        from: from_token_account.to_account_info(),
        to: token_vault.to_account_info(),
        authority: from.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(token_program.to_account_info(), cpi_accounts);
    token_interface::transfer_checked(cpi_ctx, amount, mint.decimals)?;

    Ok(())
}

#[error_code]
pub enum SplTransferOperationError {
    #[msg("Gas limit too low")]
    GasLimitTooLow,
    #[msg("Gas limit exceeded")]
    GasLimitExceeded,
    #[msg("Mint is a wrapped token")]
    MintIsAWrappedToken,
}
