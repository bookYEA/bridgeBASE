use anchor_lang::prelude::{
    borsh::{BorshDeserialize, BorshSerialize},
    *,
};
use anchor_spl::{
    token_2022::{MintToChecked, Token2022},
    token_interface::{self, Mint, TokenAccount},
};

use crate::{base_to_solana::WRAPPED_TOKEN_SEED, common::PartialTokenMetadata, ID};

#[derive(Debug, Copy, Clone, BorshSerialize, BorshDeserialize)]
pub struct FinalizeBridgeWrappedToken {
    pub local_token: Pubkey,
    pub to: Pubkey,
    pub amount: u64,
}

impl FinalizeBridgeWrappedToken {
    pub fn finalize<'info>(&self, account_infos: &'info [AccountInfo<'info>]) -> Result<()> {
        // Deserialize the accounts
        let mut iter = account_infos.iter();
        let mint = InterfaceAccount::<Mint>::try_from(next_account_info(&mut iter)?)?;
        let to_token_account =
            InterfaceAccount::<TokenAccount>::try_from(next_account_info(&mut iter)?)?;
        let token_program_2022 = Program::<Token2022>::try_from(next_account_info(&mut iter)?)?;

        // Check that the to is correct
        require!(
            to_token_account.key() == self.to,
            FinalizeBridgeWrappedTokenError::IncorrectTo
        );

        // Check that the mint is correct
        require!(
            mint.key() == self.local_token,
            FinalizeBridgeWrappedTokenError::IncorrectMint
        );

        // Get the partial token metadata
        let partial_token_metadata = PartialTokenMetadata::try_from(&mint.to_account_info())?;

        // Derive the seeds for the wrapped token mint
        let decimals_bytes = mint.decimals.to_le_bytes();
        let metadata_hash = partial_token_metadata.hash();
        let seeds: &[&[u8]] = &[
            WRAPPED_TOKEN_SEED,
            decimals_bytes.as_ref(),
            metadata_hash.as_ref(),
        ];
        let (_, mint_bump) = Pubkey::find_program_address(seeds, &ID);

        let seeds: &[&[&[u8]]] = &[&[
            WRAPPED_TOKEN_SEED,
            decimals_bytes.as_ref(),
            metadata_hash.as_ref(),
            &[mint_bump],
        ]];

        // Mint the wrapped token to the recipient
        let cpi_ctx = CpiContext::new_with_signer(
            token_program_2022.to_account_info(),
            MintToChecked {
                mint: mint.to_account_info(),
                to: to_token_account.to_account_info(),
                authority: mint.to_account_info(),
            },
            seeds,
        );
        token_interface::mint_to_checked(cpi_ctx, self.amount, mint.decimals)?;

        Ok(())
    }
}

#[error_code]
pub enum FinalizeBridgeWrappedTokenError {
    #[msg("Incorrect to")]
    IncorrectTo,
    #[msg("Incorrect mint")]
    IncorrectMint,
}
