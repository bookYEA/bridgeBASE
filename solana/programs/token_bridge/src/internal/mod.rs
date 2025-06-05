use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

use portal::{cpi as portal_cpi, program::Portal};

use crate::constants::{BRIDGE_AUTHORITY_SEED, REMOTE_BRIDGE, WRAPPED_TOKEN_SEED};

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

pub fn is_wrapped_token<'info>(
    program_id: &Pubkey,
    mint: &InterfaceAccount<'info, Mint>,
    remote_token: &[u8; 20],
) -> (bool, u8) {
    let (wrapped_token, bump) = Pubkey::find_program_address(
        &[
            WRAPPED_TOKEN_SEED,
            remote_token.as_ref(),
            mint.decimals.to_le_bytes().as_ref(),
        ],
        program_id,
    );

    (wrapped_token != mint.key(), bump)
}
