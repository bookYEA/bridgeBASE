use anchor_lang::{
    prelude::*,
    solana_program::{self, instruction::Instruction},
};

use crate::base_to_solana::{
    constants::BRIDGE_AUTHORITY_SEED, ix::Ix, state::IncomingMessage, Message, Transfer,
};

#[derive(Accounts)]
pub struct RelayMessage<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: This is the bridge authority account used to sign the external CPIs.
    #[account(seeds = [BRIDGE_AUTHORITY_SEED, message.sender.as_ref()], bump)]
    pub bridge_cpi_authority: Option<AccountInfo<'info>>,

    #[account(mut)]
    pub message: Account<'info, IncomingMessage>,
}

pub fn relay_message_handler<'a, 'info>(
    ctx: Context<'a, '_, 'info, 'info, RelayMessage<'info>>,
) -> Result<()> {
    require!(
        !ctx.accounts.message.executed,
        RelayMessageError::AlreadyExecuted
    );

    let message = Message::try_from_slice(&ctx.accounts.message.data)?;
    let (transfer, ixs) = match message {
        Message::Call(ixs) => (None, ixs),
        Message::Transfer { transfer, ixs } => (Some(transfer), ixs),
    };

    // Process the transfer if it exists
    if let Some(transfer) = transfer {
        match transfer {
            Transfer::Sol(transfer) => transfer.finalize(ctx.remaining_accounts)?,
            Transfer::Spl(transfer) => transfer.finalize(ctx.remaining_accounts)?,
            Transfer::WrappedToken(transfer) => transfer.finalize(ctx.remaining_accounts)?,
        };
    }

    let bridge_cpi_authority = ctx
        .accounts
        .bridge_cpi_authority
        .as_ref()
        .ok_or(RelayMessageError::BridgeCpiAuthorityNotFound)?;

    let bump = ctx
        .bumps
        .bridge_cpi_authority
        .ok_or(RelayMessageError::BridgeCpiAuthorityNotFound)?;

    let bridge_cpi_authority_seeds: &[&[u8]] = &[
        BRIDGE_AUTHORITY_SEED,
        ctx.accounts.message.sender.as_ref(),
        &[bump],
    ];

    // Re-add the bridge_authority to the remaining accounts
    let mut remaining_accounts = vec![bridge_cpi_authority.to_account_info()];
    remaining_accounts.extend_from_slice(ctx.remaining_accounts);

    // Process all the remaining instructions
    for ix in ixs {
        cpi(
            ix,
            bridge_cpi_authority.key(),
            &remaining_accounts,
            bridge_cpi_authority_seeds,
        )?;
    }

    ctx.accounts.message.executed = true;

    Ok(())
}

fn cpi<'info>(
    ix: Ix,
    bridge_authority_key: Pubkey,
    account_infos: &[AccountInfo<'info>],
    bridge_cpi_authority_seeds: &[&[u8]],
) -> Result<()> {
    let mut ix: Instruction = ix.into();

    let mut accounts = vec![AccountMeta::new_readonly(bridge_authority_key, true)];
    accounts.extend_from_slice(&ix.accounts);

    ix.accounts = accounts;

    solana_program::program::invoke_signed(&ix, account_infos, &[bridge_cpi_authority_seeds])?;

    Ok(())
}

#[error_code]
pub enum RelayMessageError {
    #[msg("Message already executed")]
    AlreadyExecuted,
    #[msg("Invalid transfer discriminator")]
    InvalidTransferDiscriminator,
    #[msg("Bridge CPI authority not found")]
    BridgeCpiAuthorityNotFound,
}
