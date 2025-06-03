use anchor_lang::{
    prelude::*,
    solana_program::{self, instruction::Instruction},
};

use crate::{constants::PORTAL_AUTHORITY_SEED, internal::Ix, state::RemoteCall};

#[derive(Accounts)]
pub struct RelayCall<'info> {
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
    for ix in &ixs {
        let mut ix: Instruction = ix.into();

        // Is is really needed?
        ix.accounts = ix
            .accounts
            .iter()
            .map(|acc| {
                let mut acc = acc.clone();
                if acc.pubkey == ctx.accounts.portal_authority.key() {
                    acc.is_signer = true;
                }
                acc
            })
            .collect();

        solana_program::program::invoke_signed(
            &ix,
            ctx.remaining_accounts,
            &[&[
                PORTAL_AUTHORITY_SEED,
                ctx.accounts.remote_call.sender.as_ref(),
                &[ctx.bumps.portal_authority],
            ]],
        )?;
    }

    ctx.accounts.remote_call.executed = true;

    Ok(())
}

#[error_code]
pub enum RelayCallError {
    #[msg("Already executed")]
    AlreadyExecuted,
    #[msg("Invalid sender")]
    InvalidSender,
}
