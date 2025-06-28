use anchor_lang::{
    prelude::*,
    solana_program::{self, instruction::Instruction},
};

use crate::base_to_solana::{
    constants::BRIDGE_AUTHORITY_SEED, ix::Ix, state::IncomingMessage, Message,
};

#[derive(Accounts)]
pub struct RelayMessage<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: This is the bridge authority account used to sign the external CPIs.
    #[account(seeds = [BRIDGE_AUTHORITY_SEED, message.sender.as_ref()], bump)]
    pub bridge_cpi_authority: AccountInfo<'info>,

    /// CHECK: This is the bridge authority account used to sign the self-CPI to transfer tokens.
    #[account(seeds = [BRIDGE_AUTHORITY_SEED], bump)]
    pub bridge_transfer_authority: Option<AccountInfo<'info>>,

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
    let (transfer_ix, ixs) = match message {
        Message::Call(ixs) => (None, ixs),
        Message::Transfer { transfer, ixs } => (Some(transfer), ixs),
    };

    // Process the transfer instruction if it exists
    if let Some(transfer) = transfer_ix {
        let bridge_authority_transfer = ctx
            .accounts
            .bridge_transfer_authority
            .as_ref()
            .ok_or(RelayMessageError::BridgeTransferAuthorityNotFound)?;

        let bump = ctx
            .bumps
            .bridge_transfer_authority
            .ok_or(RelayMessageError::BridgeTransferAuthorityNotFound)?;

        // Re-add the bridge authority to the remaining accounts
        let mut remaining_accounts = vec![bridge_authority_transfer.to_account_info()];
        remaining_accounts.extend_from_slice(ctx.remaining_accounts);

        cpi(
            transfer,
            bridge_authority_transfer.key(),
            &remaining_accounts,
            &[BRIDGE_AUTHORITY_SEED, &[bump]],
        )?;
    }

    let bridge_authority_seeds: &[&[u8]] = &[
        BRIDGE_AUTHORITY_SEED,
        ctx.accounts.message.sender.as_ref(),
        &[ctx.bumps.bridge_cpi_authority],
    ];

    // Re-add the bridge_authority to the remaining accounts
    let mut remaining_accounts = vec![ctx.accounts.bridge_cpi_authority.to_account_info()];
    remaining_accounts.extend_from_slice(ctx.remaining_accounts);

    // Process all the remaining instructions
    for ix in ixs {
        cpi(
            ix,
            ctx.accounts.bridge_cpi_authority.key(),
            &remaining_accounts,
            bridge_authority_seeds,
        )?;
    }

    ctx.accounts.message.executed = true;

    Ok(())
}

fn cpi<'info>(
    ix: Ix,
    bridge_authority_key: Pubkey,
    account_infos: &[AccountInfo<'info>],
    bridge_authority_seeds: &[&[u8]],
) -> Result<()> {
    let mut ix: Instruction = ix.into();

    let mut accounts = vec![AccountMeta::new_readonly(bridge_authority_key, true)];
    accounts.extend_from_slice(&ix.accounts);

    ix.accounts = accounts;

    solana_program::program::invoke_signed(&ix, account_infos, &[bridge_authority_seeds])?;

    Ok(())
}

#[error_code]
pub enum RelayMessageError {
    #[msg("Message already executed")]
    AlreadyExecuted,
    #[msg("Bridge transfer authority not found")]
    BridgeTransferAuthorityNotFound,
}
