use crate::constants::DEPOSIT_VERSION;
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
        seeds = [b"bridge_vault"],
        bump
    )]
    pub vault: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[event]
pub struct MessageSent {
    pub from: Pubkey,         // Solana address initiating the deposit
    pub to: [u8; 20],         // Target EVM address on Base
    pub version: u64,         // Version of this deposit transaction event
    pub opaque_data: Vec<u8>, // Data payload for the Base EVM call
}

pub fn deposit_transaction_handler(
    ctx: Context<DepositTransaction>,
    to: [u8; 20],
    value: u64,
    gas_limit: u64,
    is_creation: bool,
    data: Vec<u8>,
) -> Result<()> {
    if value > 0 {
        // Transfer lamports from user to vault PDA
        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.user.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
            },
        );
        anchor_lang::system_program::transfer(cpi_context, value)?;
    }

    // Just to be safe, make sure that people specify address(0) as the target when doing
    // contract creations.
    if is_creation && to != [0; 20] {
        return err!(DepositTransactionError::BadTarget);
    }

    // Prevent depositing transactions that have too small of a gas limit. Users should pay
    // more for more resource usage.
    if gas_limit < minimum_gas_limit(data.len() as u64) {
        return err!(DepositTransactionError::GasLimitTooLow);
    }

    // Compute the opaque data that will be emitted as part of the TransactionDeposited event.
    // We use opaque data so that we can update the TransactionDeposited event in the future
    // without breaking the current interface.
    let opaque_data = encode_packed(value, value, gas_limit, is_creation, data);

    // Emit event for the relayer
    emit!(MessageSent {
        from: ctx.accounts.user.key(),
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
    opaque_data.extend_from_slice(&msg_value.to_le_bytes()); // Equivalent to msg.value in Solidity
    opaque_data.extend_from_slice(&value.to_le_bytes()); // Equivalent to _value
    opaque_data.extend_from_slice(&gas_limit.to_le_bytes()); // Equivalent to _gasLimit
    opaque_data.push(is_creation as u8); // Equivalent to _isCreation
    opaque_data.extend_from_slice(&data); // Equivalent to _data
    return opaque_data;
}

#[error_code]
pub enum DepositTransactionError {
    #[msg("Bad target")]
    BadTarget,
    #[msg("Gas limit too low")]
    GasLimitTooLow,
}
