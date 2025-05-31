use crate::{
    BASE_TRANSACTION_COST, DEPOSIT_VERSION, GAS_FEE_RECEIVER, GAS_PER_BYTE_COST, SOL_TO_ETH_FACTOR,
};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct DepositTransaction<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut, address = GAS_FEE_RECEIVER)]
    /// CHECK: This is the hardcoded gas fee receiver account.
    pub gas_fee_receiver: AccountInfo<'info>,

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
/// @param _gasLimit   Amount of L2 gas to purchase by burning gas on L1.
/// @param _isCreation Whether or not the transaction is a contract creation.
/// @param _data       Data to trigger the recipient with.
pub fn deposit_transaction_handler(
    ctx: Context<DepositTransaction>,
    to: [u8; 20],
    gas_limit: u64,
    is_creation: bool,
    data: Vec<u8>,
) -> Result<()> {
    deposit_transaction_internal(
        &ctx.accounts.system_program,
        &ctx.accounts.user,
        &ctx.accounts.gas_fee_receiver,
        ctx.accounts.user.key(),
        to,
        gas_limit,
        is_creation,
        &data,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn deposit_transaction_internal<'info>(
    system_program: &Program<'info, System>,
    gas_fee_payer: &Signer<'info>,
    gas_fee_receiver: &AccountInfo<'info>,
    from: Pubkey,
    to: [u8; 20],
    gas_limit: u64,
    is_creation: bool,
    data: &[u8],
) -> Result<()> {
    // Just to be safe, make sure that people specify address(0) as the target when doing
    // contract creations.
    require!(!is_creation || to == [0; 20], PortalError::BadTarget);

    // Prevent depositing transactions that have too small of a gas limit. Users should pay
    // more for more resource usage.
    require!(
        gas_limit >= minimum_gas_limit(data.len() as u64),
        PortalError::GasLimitTooLow
    );

    // Compute the opaque data that will be emitted as part of the TransactionDeposited event.
    // We use opaque data so that we can update the TransactionDeposited event in the future
    // without breaking the current interface.
    let opaque_data = encode_packed(gas_limit, is_creation, data);

    // Gas metering
    meter_gas(system_program, gas_fee_payer, gas_fee_receiver, gas_limit)?;

    // Emit event for the relayer
    emit!(TransactionDeposited {
        from: from.key(),
        to,
        version: DEPOSIT_VERSION,
        opaque_data,
    });

    Ok(())
}

fn minimum_gas_limit(byte_count: u64) -> u64 {
    byte_count * GAS_PER_BYTE_COST + BASE_TRANSACTION_COST
}

fn encode_packed(gas_limit: u64, is_creation: bool, data: &[u8]) -> Vec<u8> {
    let mut opaque_data = Vec::new();
    opaque_data.extend_from_slice(&gas_limit.to_be_bytes());
    opaque_data.push(is_creation as u8);
    opaque_data.extend_from_slice(data);
    opaque_data
}

fn meter_gas<'info>(
    system_program: &Program<'info, System>,
    gas_fee_payer: &Signer<'info>,
    gas_fee_receiver: &AccountInfo<'info>,
    gas_limit: u64,
) -> Result<()> {
    let base_fee = gas_base_fee();
    let gas_cost = gas_limit * base_fee * SOL_TO_ETH_FACTOR;

    let cpi_context = CpiContext::new(
        system_program.to_account_info(),
        anchor_lang::system_program::Transfer {
            from: gas_fee_payer.to_account_info(),
            to: gas_fee_receiver.clone(),
        },
    );
    anchor_lang::system_program::transfer(cpi_context, gas_cost)?;

    Ok(())
}

fn gas_base_fee() -> u64 {
    // TODO: Use VRGDA or equivalent.
    30 // 30 gwei expressed in lamports
}

#[error_code]
pub enum PortalError {
    #[msg("Bad target")]
    BadTarget,
    #[msg("Gas limit too low")]
    GasLimitTooLow,
}
