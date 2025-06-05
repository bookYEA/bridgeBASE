use anchor_lang::prelude::*;

use crate::constants::{
    BASE_TRANSACTION_COST, GAS_FEE_RECEIVER, GAS_PER_BYTE_COST, SOL_TO_ETH_FACTOR,
};

use super::Call;

#[derive(Accounts)]
pub struct SendCall<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK: This is the hardcoded gas fee receiver account.
    #[account(mut, address = GAS_FEE_RECEIVER @ SendCallError::IncorrectGasFeeReceiver)]
    pub gas_fee_receiver: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[event]
pub struct CallSent {
    pub from: Pubkey,
    pub to: [u8; 20],
    pub opaque_data: Vec<u8>,
}

pub fn send_call_handler(
    ctx: Context<SendCall>,
    to: [u8; 20],
    gas_limit: u64,
    is_creation: bool,
    data: Vec<u8>,
) -> Result<()> {
    send_call(
        &ctx.accounts.system_program,
        &ctx.accounts.payer,
        &ctx.accounts.gas_fee_receiver,
        Call {
            from: ctx.accounts.authority.key(),
            to,
            gas_limit,
            is_creation,
            data,
        },
    )
}

pub fn send_call<'info>(
    system_program: &Program<'info, System>,
    payer: &Signer<'info>,
    gas_fee_receiver: &AccountInfo<'info>,
    call: Call,
) -> Result<()> {
    let Call {
        from,
        to,
        gas_limit,
        is_creation,
        data,
    } = call;

    require!(!is_creation || to == [0; 20], SendCallError::BadTarget);
    require!(
        gas_limit >= minimum_gas_limit(&data),
        SendCallError::GasLimitTooLow
    );

    let opaque_data = {
        let mut opaque_data = vec![];
        opaque_data.extend_from_slice(&gas_limit.to_le_bytes());
        opaque_data.push(is_creation as u8);
        opaque_data.extend_from_slice(&data);
        opaque_data
    };

    meter_gas(system_program, payer, gas_fee_receiver, gas_limit)?;

    emit!(CallSent {
        from,
        to,
        opaque_data,
    });

    Ok(())
}

fn minimum_gas_limit(data: &[u8]) -> u64 {
    data.len() as u64 * GAS_PER_BYTE_COST + BASE_TRANSACTION_COST
}

fn meter_gas<'info>(
    system_program: &Program<'info, System>,
    payer: &Signer<'info>,
    gas_fee_receiver: &AccountInfo<'info>,
    gas_limit: u64,
) -> Result<()> {
    let base_fee = gas_base_fee();
    let gas_cost = gas_limit * base_fee * SOL_TO_ETH_FACTOR;

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

fn gas_base_fee() -> u64 {
    // TODO: Use VRGDA or equivalent.
    30 // 30 gwei expressed in lamports
}

#[error_code]
pub enum SendCallError {
    #[msg("Incorrect gas fee receiver")]
    IncorrectGasFeeReceiver,
    #[msg("Bad target")]
    BadTarget,
    #[msg("Gas limit too low")]
    GasLimitTooLow,
}
