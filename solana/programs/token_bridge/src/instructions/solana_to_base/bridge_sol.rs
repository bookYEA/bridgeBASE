use alloy_primitives::{FixedBytes, U256};
use alloy_sol_types::SolCall;
use anchor_lang::{
    prelude::*,
    system_program::{self, Transfer},
};
use portal::{cpi as portal_cpi, program::Portal};

use crate::{
    constants::{BRIDGE_AUTHORITY_SEED, NATIVE_SOL_PUBKEY, SOL_VAULT_SEED},
    internal::cpi_send_call,
    solidity::Bridge,
};

#[derive(Accounts)]
#[instruction(remote_token: [u8; 20])]
pub struct BridgeSol<'info> {
    // Bridge accounts
    #[account(mut)]
    pub from: Signer<'info>,

    /// CHECK: This is the sol vault account for a specific remote token.
    #[account(mut, seeds = [SOL_VAULT_SEED, remote_token.as_ref()], bump)]
    pub sol_vault: AccountInfo<'info>,

    pub portal: Program<'info, Portal>,

    // Portal remaining accounts
    /// CHECK: Checked by the Portal program that we CPI into.
    #[account(mut)]
    pub messenger: AccountInfo<'info>,

    /// CHECK: This is the Bridge authority account.
    ///        It is used as the authority when CPIing to the Portal program.
    #[account(seeds = [BRIDGE_AUTHORITY_SEED], bump)]
    pub bridge_authority: AccountInfo<'info>,

    /// CHECK: Checked by the Portal program that we CPI into.
    #[account(mut)]
    pub gas_fee_receiver: AccountInfo<'info>,

    /// CHECK: Checked by the Portal program that we CPI into.
    #[account(mut)]
    pub eip1559: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

pub fn bridge_sol_handler(
    ctx: Context<BridgeSol>,
    remote_token: [u8; 20],
    to: [u8; 20],
    amount: u64,
    min_gas_limit: u64,
    extra_data: Vec<u8>,
) -> Result<()> {
    lock_sol(&ctx, amount)?;

    cpi_send_call(
        &ctx.accounts.portal,
        portal_cpi::accounts::SendCall {
            payer: ctx.accounts.from.to_account_info(),
            authority: ctx.accounts.bridge_authority.to_account_info(),
            gas_fee_receiver: ctx.accounts.gas_fee_receiver.to_account_info(),
            eip1559: ctx.accounts.eip1559.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
        },
        ctx.bumps.bridge_authority,
        min_gas_limit,
        Bridge::finalizeBridgeTokenCall {
            localToken: remote_token.into(), // NOTE: Intentionally flip the tokens so that when executing on Base it's correct.
            remoteToken: FixedBytes::from(NATIVE_SOL_PUBKEY.to_bytes()), // NOTE: Intentionally flip the tokens so that when executing on Base it's correct.
            from: FixedBytes::from(ctx.accounts.from.key().to_bytes()),
            to: to.into(),
            amount: U256::from(amount),
            extraData: extra_data.into(),
        }
        .abi_encode(),
    )
}

fn lock_sol(ctx: &Context<BridgeSol>, amount: u64) -> Result<()> {
    let cpi_ctx = CpiContext::new(
        ctx.accounts.system_program.to_account_info(),
        Transfer {
            from: ctx.accounts.from.to_account_info(),
            to: ctx.accounts.sol_vault.to_account_info(),
        },
    );
    system_program::transfer(cpi_ctx, amount)
}
