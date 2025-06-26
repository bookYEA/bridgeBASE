use anchor_lang::{
    prelude::*,
    solana_program::{self, instruction::Instruction},
};

use crate::base_to_solana::{constants::BRIDGE_AUTHORITY_SEED, ix::Ix, state::IncomingMessage};

#[derive(Accounts)]
pub struct RelayMessage<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: This is the portal authority account.
    #[account(seeds = [BRIDGE_AUTHORITY_SEED, message.sender.as_ref()], bump)]
    pub bridge_authority: AccountInfo<'info>,

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

    let ixs = Vec::<Ix>::try_from_slice(&ctx.accounts.message.data)?;

    let bridge_authority_seeds: &[&[u8]] = &[
        BRIDGE_AUTHORITY_SEED,
        ctx.accounts.message.sender.as_ref(),
        &[ctx.bumps.bridge_authority],
    ];

    // Re-add the bridge_authority to the remaining accounts
    let mut remaining_accounts = vec![ctx.accounts.bridge_authority.to_account_info()];
    remaining_accounts.extend_from_slice(ctx.remaining_accounts);

    for ix in ixs {
        let mut ix: Instruction = ix.into();

        let mut accounts = vec![AccountMeta::new_readonly(
            ctx.accounts.bridge_authority.key(),
            true,
        )];
        accounts.extend_from_slice(&ix.accounts);

        ix.accounts = accounts;

        solana_program::program::invoke_signed(
            &ix,
            &remaining_accounts,
            &[bridge_authority_seeds],
        )?;
    }

    ctx.accounts.message.executed = true;

    Ok(())
}

#[error_code]
pub enum RelayMessageError {
    #[msg("Already executed")]
    AlreadyExecuted,
}
