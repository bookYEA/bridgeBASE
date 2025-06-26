use anchor_lang::{
    prelude::*,
    solana_program::{self, instruction::Instruction},
};

use crate::{constants::PORTAL_AUTHORITY_SEED, internal::Ix, state::RemoteCall};

#[derive(Accounts)]
pub struct RelayCall<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: This is the portal authority account.
    #[account(seeds = [PORTAL_AUTHORITY_SEED, remote_call.sender.as_ref()], bump)]
    pub portal_authority: AccountInfo<'info>,

    #[account(mut)]
    pub remote_call: Account<'info, RemoteCall>,
}

pub fn relay_call_handler<'a, 'info>(
    ctx: Context<'a, '_, 'info, 'info, RelayCall<'info>>,
) -> Result<()> {
    require!(
        !ctx.accounts.remote_call.executed,
        RelayCallError::AlreadyExecuted
    );

    let ixs = Vec::<Ix>::try_from_slice(&ctx.accounts.remote_call.data)?;

    let portal_authority_seeds: &[&[u8]] = &[
        PORTAL_AUTHORITY_SEED,
        ctx.accounts.remote_call.sender.as_ref(),
        &[ctx.bumps.portal_authority],
    ];

    // Re-add the portal_authority to the remaining accounts
    let mut remaining_accounts = vec![ctx.accounts.portal_authority.to_account_info()];
    remaining_accounts.extend_from_slice(ctx.remaining_accounts);

    for ix in ixs {
        let mut ix: Instruction = ix.into();

        let mut accounts = vec![AccountMeta::new_readonly(
            ctx.accounts.portal_authority.key(),
            true,
        )];
        accounts.extend_from_slice(&ix.accounts);

        ix.accounts = accounts;

        solana_program::program::invoke_signed(
            &ix,
            &remaining_accounts,
            &[portal_authority_seeds],
        )?;
    }

    ctx.accounts.remote_call.executed = true;

    Ok(())
}

#[error_code]
pub enum RelayCallError {
    #[msg("Already executed")]
    AlreadyExecuted,
}
