use crate::{constants::DEPOSIT_VERSION, VAULT_SEED};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct DepositTransaction<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK: This is the vault PDA. We are only using it to transfer SOL via CPI
    /// to the system program, so no data checks are required. The address is
    /// verified by the seeds constraint.
    #[account(
        mut,
        seeds = [VAULT_SEED],
        bump
    )]
    pub vault: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[event]
// Emitted when a transaction is deposited from L1 to L2. The parameters of this event
// are read by the rollup node and used to derive deposit transactions on L2.
pub struct TransactionDeposited {
    pub from: Pubkey,         // Solana key that triggered the deposit transaction.
    pub to: [u8; 20],         // Target EVM address on Base
    pub version: u64,         // Version of this deposit transaction event
    pub opaque_data: Vec<u8>, // ABI encoded deposit data to be parsed offchain.
}

/// @notice Accepts deposits of SOL and data, and emits a TransactionDeposited event for use in
///         deriving deposit transactions. Consider using the CrossDomainMessenger contracts for
///         a simpler developer experience.
///
/// @param _to         Target address on L2.
/// @param _value      SOL value to send to the recipient.
/// @param _gasLimit   Amount of L2 gas to purchase by burning gas on L1.
/// @param _isCreation Whether or not the transaction is a contract creation.
/// @param _data       Data to trigger the recipient with.
pub fn deposit_transaction_handler(
    ctx: Context<DepositTransaction>,
    to: [u8; 20],
    value: u64,
    gas_limit: u64,
    is_creation: bool,
    data: Vec<u8>,
) -> Result<()> {
    deposit_transaction_internal(
        &ctx.accounts.system_program,
        &ctx.accounts.user.to_account_info(),
        &ctx.accounts.vault.to_account_info(),
        ctx.accounts.user.key(),
        to,
        value,
        gas_limit,
        is_creation,
        data,
    )
}

pub fn deposit_transaction_internal<'info>(
    system_program: &Program<'info, System>,
    user: &AccountInfo<'info>,
    vault: &AccountInfo<'info>,
    from: Pubkey,
    to: [u8; 20],
    value: u64,
    gas_limit: u64,
    is_creation: bool,
    data: Vec<u8>,
) -> Result<()> {
    if value > 0 {
        // Transfer lamports from user to vault PDA
        let cpi_context = CpiContext::new(
            system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: user.clone(),
                to: vault.clone(),
            },
        );
        anchor_lang::system_program::transfer(cpi_context, value)?;
    }

    // Just to be safe, make sure that people specify address(0) as the target when doing
    // contract creations.
    if is_creation && to != [0; 20] {
        return err!(PortalError::BadTarget);
    }

    // Prevent depositing transactions that have too small of a gas limit. Users should pay
    // more for more resource usage.
    if gas_limit < minimum_gas_limit(data.len() as u64) {
        return err!(PortalError::GasLimitTooLow);
    }

    // Compute the opaque data that will be emitted as part of the TransactionDeposited event.
    // We use opaque data so that we can update the TransactionDeposited event in the future
    // without breaking the current interface.
    let opaque_data = encode_packed(value, value, gas_limit, is_creation, data);

    // Emit event for the relayer
    emit!(TransactionDeposited {
        from: from,
        to,
        version: DEPOSIT_VERSION,
        opaque_data,
    });

    Ok(())
}

fn minimum_gas_limit(byte_count: u64) -> u64 {
    return byte_count * 40 + 21000;
}

fn encode_packed(
    msg_value: u64,
    value: u64,
    gas_limit: u64,
    is_creation: bool,
    data: Vec<u8>,
) -> Vec<u8> {
    let mut opaque_data = Vec::new();
    opaque_data.extend_from_slice(&msg_value.to_be_bytes()); // Equivalent to msg.value in Solidity
    opaque_data.extend_from_slice(&value.to_be_bytes()); // Equivalent to _value
    opaque_data.extend_from_slice(&gas_limit.to_be_bytes()); // Equivalent to _gasLimit
    opaque_data.push(is_creation as u8); // Equivalent to _isCreation
    opaque_data.extend_from_slice(&data); // Equivalent to _data
    return opaque_data;
}

#[error_code]
pub enum PortalError {
    #[msg("Bad target")]
    BadTarget,
    #[msg("Gas limit too low")]
    GasLimitTooLow,
}
