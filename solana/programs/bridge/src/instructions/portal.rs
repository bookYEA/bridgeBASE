use crate::{
    BASE_TRANSACTION_COST, DEPOSIT_VERSION, GAS_FEE_RECEIVER, GAS_PER_BYTE_COST, SOL_TO_ETH_FACTOR,
};
use anchor_lang::prelude::*;

/// Accounts required for depositing a transaction from Solana to Base.
#[derive(Accounts)]
pub struct DepositTransaction<'info> {
    /// The user account that is initiating the deposit transaction.
    /// Must be mutable as SOL will be transferred from this account to pay gas fees.
    #[account(mut)]
    pub user: Signer<'info>,

    /// The account that receives gas fees for processing the deposit transaction.
    /// This is a hardcoded address defined by GAS_FEE_RECEIVER constant.
    /// Must be mutable as it will receive SOL transfers for gas payments.
    #[account(mut, address = GAS_FEE_RECEIVER)]
    /// CHECK: This is the hardcoded gas fee receiver account.
    pub gas_fee_receiver: AccountInfo<'info>,

    /// The Solana system program used for transferring SOL for gas fees.
    pub system_program: Program<'info, System>,
}

/// Event emitted when a transaction is deposited from Solana to Base.
///
/// This event contains all the necessary information for the oracle to derive
/// the corresponding deposit transaction on Base. The oracle listens for these
/// events and processes them to create transactions on Base.
#[event]
pub struct TransactionDeposited {
    /// The Solana public key of the account that triggered the deposit transaction.
    pub from: Pubkey,
    /// The target EVM address on Base (20 bytes) where the transaction should be sent.
    pub to: [u8; 20],
    /// Version of this deposit transaction event format for future compatibility.
    pub version: u64,
    /// ABI-encoded deposit data containing gas limit, creation flag, and transaction data.
    /// This opaque format allows for future updates without breaking the interface.
    pub opaque_data: Vec<u8>,
}

/// Accepts transaction data, emitting a TransactionDeposited event for use in deriving deposit
/// transactions on Base.
///
/// This is the main entry point for depositing transactions from Solana to Base.
/// Users call this function to bridge transactions, paying gas fees in SOL which
/// are converted to the appropriate gas cost for Base execution.
///
/// Consider using the messenger component for a simpler developer experience when building
/// applications that need to communicate between Solana and Base.
///
/// # Arguments
///
/// * `ctx`         - The account context containing user, gas fee receiver, and system program
/// * `to`          - Target EVM address on Base
/// * `gas_limit`   - Amount of Base gas to purchase by paying gas fees on L1
/// * `is_creation` - Whether the transaction is a contract creation (target should be zero address)
/// * `data`        - Transaction data to execute on Base
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

/// Internal implementation of deposit transaction processing.
///
/// This function contains the core logic for validating deposit parameters,
/// calculating gas costs, transferring fees, and emitting the deposit event.
/// It's separated from the handler to allow for potential reuse in other contexts.
///
/// # Arguments
///
/// * `system_program`   - The Solana system program for SOL transfers
/// * `gas_fee_payer`    - The account that will pay gas fees in SOL
/// * `gas_fee_receiver` - The account that will receive the gas fee payments
/// * `from`             - The originating Solana public key for the deposit
/// * `to`               - Target EVM address on L2
/// * `gas_limit`        - Amount of L2 gas to purchase
/// * `is_creation`      - Whether this is a contract creation transaction
/// * `data`             - Transaction data to execute on L2
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

/// Calculates the minimum gas limit required for a transaction based on its data size.
///
/// The minimum gas limit ensures that transactions pay for the computational resources
/// they consume, including both base transaction costs and per-byte data costs.
///
/// # Arguments
///
/// * `byte_count` - The number of bytes in the transaction data
fn minimum_gas_limit(byte_count: u64) -> u64 {
    byte_count * GAS_PER_BYTE_COST + BASE_TRANSACTION_COST
}

/// Encodes deposit transaction parameters into a packed byte format.
///
/// This function creates the opaque data that will be included in the TransactionDeposited
/// event. The packed format allows the oracle to decode the transaction parameters
/// on Base. Using an opaque format provides flexibility for future protocol updates.
///
/// # Format
///
/// * Bytes 0-7: gas_limit (u64, big-endian)
/// * Byte 8: is_creation flag (0 or 1)
/// * Bytes 9+: transaction data
///
/// # Arguments
///
/// * `gas_limit`   - The L2 gas limit for the transaction
/// * `is_creation` - Whether this is a contract creation transaction
/// * `data`        - The transaction data to execute on L2
fn encode_packed(gas_limit: u64, is_creation: bool, data: &[u8]) -> Vec<u8> {
    let mut opaque_data = Vec::new();
    opaque_data.extend_from_slice(&gas_limit.to_be_bytes());
    opaque_data.push(is_creation as u8);
    opaque_data.extend_from_slice(data);
    opaque_data
}

/// Handles gas fee payment by transferring SOL from the payer to the fee receiver.
///
/// This function calculates the cost of gas in SOL based on the current base fee
/// and transfers the appropriate amount from the gas fee payer to the receiver.
/// The gas cost is calculated as: gas_limit * base_fee * SOL_TO_ETH_FACTOR
///
/// # Arguments
///
/// * `system_program`   - The Solana system program for executing the transfer
/// * `gas_fee_payer`    - The account that will pay the gas fees
/// * `gas_fee_receiver` - The account that will receive the gas fee payment
/// * `gas_limit`        - The amount of Base gas being purchased
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

/// Returns the current base fee for gas pricing.
///
/// Currently returns a fixed rate of 30 gwei (expressed in lamports).
/// In the future, this should be replaced with a dynamic pricing mechanism
/// such as VRGDA (Variable Rate Gradual Dutch Auction) or equivalent.
fn gas_base_fee() -> u64 {
    // TODO: Use VRGDA or equivalent.
    30 // 30 gwei expressed in lamports
}

/// Error types that can occur during portal operations.
#[error_code]
pub enum PortalError {
    /// Thrown when a contract creation transaction has a non-zero target address.
    /// Contract creation transactions must specify the zero address as the target.
    #[msg("Bad target")]
    BadTarget,
    /// Thrown when the specified gas limit is below the minimum required amount.
    /// The minimum is calculated based on transaction data size and base costs.
    #[msg("Gas limit too low")]
    GasLimitTooLow,
}
