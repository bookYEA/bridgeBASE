use anchor_lang::prelude::*;
use anchor_lang::solana_program::keccak;
use anchor_spl::{
    token_2022::spl_token_2022::{
        extension::{BaseStateWithExtensions, PodStateWithExtensions},
        pod::PodMint,
    },
    token_interface::spl_token_metadata_interface::state::TokenMetadata,
};

pub mod finalize_bridge_sol;
pub mod finalize_bridge_spl;
pub mod finalize_bridge_token;
pub mod wrap_token;

pub use finalize_bridge_sol::*;
pub use finalize_bridge_spl::*;
pub use finalize_bridge_token::*;
pub use wrap_token::*;

use crate::constants::REMOTE_TOKEN_METADATA_KEY;

#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct PartialTokenMetadata {
    pub remote_token: [u8; 20],
    pub name: String,
    pub symbol: String,
}

impl From<&PartialTokenMetadata> for TokenMetadata {
    fn from(value: &PartialTokenMetadata) -> Self {
        TokenMetadata {
            name: value.name.clone(),
            symbol: value.symbol.clone(),
            additional_metadata: vec![(
                REMOTE_TOKEN_METADATA_KEY.to_string(),
                hex::encode(value.remote_token),
            )],
            ..Default::default()
        }
    }
}

impl TryFrom<TokenMetadata> for PartialTokenMetadata {
    type Error = Error;

    fn try_from(metadata: TokenMetadata) -> Result<Self> {
        let (key, value) = metadata
            .additional_metadata
            .first()
            .ok_or(TokenMetadataError::RemoteTokenNotFound)?;

        require!(
            key == REMOTE_TOKEN_METADATA_KEY,
            TokenMetadataError::RemoteTokenNotFound
        );

        let remote_token =
            hex::decode(value).map_err(|_| TokenMetadataError::RemoteTokenNotFound)?;
        let remote_token = <[u8; 20]>::try_from(remote_token)
            .map_err(|_| TokenMetadataError::InvalidRemoteToken)?;

        Ok(PartialTokenMetadata {
            remote_token,
            name: metadata.name,
            symbol: metadata.symbol,
        })
    }
}

impl TryFrom<&AccountInfo<'_>> for PartialTokenMetadata {
    type Error = Error;

    fn try_from(value: &AccountInfo<'_>) -> Result<Self> {
        let token_metadata = mint_info_to_token_metadata(value)?;
        Self::try_from(token_metadata)
    }
}

impl PartialTokenMetadata {
    pub fn hash(&self) -> [u8; 32] {
        let mut data = Vec::new();
        data.extend_from_slice(self.remote_token.as_ref());
        data.extend_from_slice(self.name.as_bytes());
        data.extend_from_slice(self.symbol.as_bytes());
        keccak::hash(&data).0
    }
}

pub fn mint_info_to_token_metadata(mint: &AccountInfo<'_>) -> Result<TokenMetadata> {
    let mint_data = mint.data.borrow();
    let mint_with_extension = PodStateWithExtensions::<PodMint>::unpack(&mint_data)?;
    let token_metadata = mint_with_extension.get_variable_len_extension::<TokenMetadata>()?;
    Ok(token_metadata)
}

#[error_code]
pub enum TokenMetadataError {
    #[msg("Invalid remote token")]
    RemoteTokenNotFound,
    #[msg("Invalid remote token")]
    InvalidRemoteToken,
}
