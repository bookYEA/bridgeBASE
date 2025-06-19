use anchor_lang::prelude::*;

use portal::{
    cpi as portal_cpi,
    instructions::{Call, CallType},
    program::Portal,
};

use crate::constants::{BRIDGE_AUTHORITY_SEED, REMOTE_BRIDGE};

pub mod metadata;

pub fn cpi_send_call<'info>(
    portal: &Program<'info, Portal>,
    accounts: portal_cpi::accounts::SendCall<'info>,
    authority_bump: u8,
    gas_limit: u64,
    data: Vec<u8>,
) -> Result<()> {
    let cpi_program = portal.to_account_info();

    let seeds: &[&[&[u8]]] = &[&[BRIDGE_AUTHORITY_SEED, &[authority_bump]]];
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, accounts, seeds);
    portal_cpi::send_call(
        cpi_ctx,
        Call {
            ty: CallType::Call,
            to: REMOTE_BRIDGE,
            gas_limit,
            data,
        },
    )?;

    Ok(())
}
