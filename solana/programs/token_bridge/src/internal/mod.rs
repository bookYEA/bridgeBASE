use anchor_lang::prelude::*;
use portal::{cpi as portal_cpi, program::Portal};

use crate::constants::{BRIDGE_AUTHORITY_SEED, REMOTE_BRIDGE};

pub fn cpi_send_message<'info>(
    portal: &Program<'info, Portal>,
    accounts: portal_cpi::accounts::SendMessage<'info>,
    authority_bump: u8,
    message: Vec<u8>,
    min_gas_limit: u64,
) -> Result<()> {
    let cpi_program = portal.to_account_info();

    let seeds: &[&[&[u8]]] = &[&[BRIDGE_AUTHORITY_SEED, &[authority_bump]]];
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, accounts, seeds);
    portal_cpi::send_message(cpi_ctx, REMOTE_BRIDGE, message, min_gas_limit)?;

    Ok(())
}
