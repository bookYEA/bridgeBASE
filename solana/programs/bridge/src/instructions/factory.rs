use anchor_lang::prelude::*;
use anchor_spl::{token_interface::Mint, token_interface::TokenInterface};

use crate::MINT_SEED;

/// Instruction accounts for creating a new mint account that represents a bridged token
///
/// This instruction creates a PDA mint account that corresponds to a token from another
/// blockchain (identified by remote_token). The mint serves as the Solana representation
/// of the remote token in the bridge system.
#[derive(Accounts)]
#[instruction(remote_token: [u8; 20], decimals: u8)]
pub struct CreateMint<'info> {
    /// The account that will pay for the mint creation and sign the transaction
    #[account(mut)]
    pub signer: Signer<'info>,

    /// The mint account being created as a PDA
    ///
    /// This mint represents a bridged token on Solana. Key properties:
    /// - Decimals are set based on the remote token's decimals (may be clamped to 9 for Solana compatibility)
    /// - Both mint authority and freeze authority are set to the mint itself
    /// - Seeds ensure each remote token + decimals combination has a unique mint
    /// - The mint can be used to mint/burn tokens as part of bridge operations
    #[account(
        init,
        payer = signer,
        mint::decimals = decimals, // TODO: Decimals might need to be clamped to 9.
        mint::authority = mint,
        mint::freeze_authority = mint,
        seeds = [MINT_SEED, remote_token.as_ref(), decimals.to_le_bytes().as_ref()],
        bump
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    /// SPL Token program interface for mint operations
    pub token_program: Interface<'info, TokenInterface>,

    /// System program for creating the mint account
    pub system_program: Program<'info, System>,
}

/// Creates a new mint account for a bridged token
///
/// This function initializes a mint that represents a token from another blockchain
/// within the Solana bridge ecosystem.
///
/// # Arguments
/// * `ctx` - The instruction context containing all required accounts
pub fn create_mint_handler(ctx: Context<CreateMint>) -> Result<()> {
    msg!("Created Mint Account: {:?}", ctx.accounts.mint.key());
    Ok(())
}
