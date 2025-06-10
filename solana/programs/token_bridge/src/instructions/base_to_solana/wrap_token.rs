use anchor_lang::prelude::*;
use anchor_lang::solana_program::rent::{
    DEFAULT_EXEMPTION_THRESHOLD, DEFAULT_LAMPORTS_PER_BYTE_YEAR,
};
use anchor_lang::system_program::{transfer, Transfer};
use anchor_spl::token_interface::{
    spl_token_metadata_interface::state::{Field, TokenMetadata},
    token_metadata_initialize, token_metadata_update_field, Mint, Token2022,
    TokenMetadataInitialize, TokenMetadataUpdateField,
};

use crate::constants::{REMOTE_TOKEN_METADATA_KEY, WRAPPED_TOKEN_SEED};
use crate::instructions::PartialTokenMetadata;

#[derive(Accounts)]
#[instruction(decimals: u8, metadata: PartialTokenMetadata)]
pub struct WrapToken<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        seeds = [
            WRAPPED_TOKEN_SEED,
            decimals.to_le_bytes().as_ref(),
            metadata.hash().as_ref(),
        ],
        bump,
        mint::decimals = decimals,
        mint::authority = mint,
        mint::freeze_authority = mint,
        // extensions::metadata_pointer::authority = mint,
        extensions::metadata_pointer::metadata_address = mint,
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

pub fn wrap_token_handler(
    ctx: Context<WrapToken>,
    decimals: u8,
    partial_token_metadata: PartialTokenMetadata,
) -> Result<()> {
    require!(decimals <= 9, WrapTokenError::InvalidDecimals);

    let token_metadata = TokenMetadata::from(&partial_token_metadata);

    // FIXME: Computation is most likely unaccurate
    // Calculate lamports required for the additional metadata
    let data_len = token_metadata.tlv_size_of()?;
    let lamports =
        data_len as u64 * DEFAULT_LAMPORTS_PER_BYTE_YEAR * DEFAULT_EXEMPTION_THRESHOLD as u64;

    // Transfer additional lamports to mint account
    transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.payer.to_account_info(),
                to: ctx.accounts.mint.to_account_info(),
            },
        ),
        lamports,
    )?;

    let decimals_bytes = decimals.to_le_bytes();
    let metadata_hash = partial_token_metadata.hash();

    let seeds = &[
        WRAPPED_TOKEN_SEED,
        decimals_bytes.as_ref(),
        metadata_hash.as_ref(),
        &[ctx.bumps.mint],
    ];

    // Initialize token metadata
    token_metadata_initialize(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            TokenMetadataInitialize {
                program_id: ctx.accounts.token_program.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                metadata: ctx.accounts.mint.to_account_info(),
                mint_authority: ctx.accounts.mint.to_account_info(),
                update_authority: ctx.accounts.mint.to_account_info(),
            },
        )
        .with_signer(&[seeds]),
        token_metadata.name,
        token_metadata.symbol,
        Default::default(),
    )?;

    // Set the remote token metadata key
    token_metadata_update_field(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            TokenMetadataUpdateField {
                program_id: ctx.accounts.token_program.to_account_info(),
                metadata: ctx.accounts.mint.to_account_info(),
                update_authority: ctx.accounts.mint.to_account_info(),
            },
        )
        .with_signer(&[seeds]),
        Field::Key(REMOTE_TOKEN_METADATA_KEY.to_string()),
        hex::encode(partial_token_metadata.remote_token),
    )?;

    Ok(())
}

#[error_code]
pub enum WrapTokenError {
    #[msg("Invalid decimals")]
    InvalidDecimals,
}

#[cfg(test)]
mod tests {
    use anchor_lang::{
        prelude::*, solana_program::native_token::LAMPORTS_PER_SOL, InstructionData,
    };
    use anchor_spl::{
        token_2022::spl_token_2022::{
            extension::{
                metadata_pointer::MetadataPointer, BaseStateWithExtensions, StateWithExtensions,
            },
            state::Mint,
        },
        token_interface::spl_token_metadata_interface::state::TokenMetadata,
    };
    use litesvm::LiteSVM;
    use solana_instruction::Instruction;
    use solana_keypair::Keypair;
    use solana_message::Message;
    use solana_signer::Signer;
    use solana_transaction::Transaction;

    use crate::{
        constants::{REMOTE_TOKEN_METADATA_KEY, WRAPPED_TOKEN_SEED},
        instructions::PartialTokenMetadata,
        test_utils::SPL_TOKEN_PROGRAM_ID,
        ID as TOKEN_BRIDGE_PROGRAM_ID,
    };

    #[test]
    fn test_wrap_token_success() {
        let mut svm = LiteSVM::new();
        svm.add_program_from_file(
            TOKEN_BRIDGE_PROGRAM_ID,
            "../../target/deploy/token_bridge.so",
        )
        .unwrap();

        // Test parameters
        let partial_token_metadata = PartialTokenMetadata {
            remote_token: [0x42u8; 20],
            name: "Wrapped USDC".to_string(),
            symbol: "WUSDC".to_string(),
        };
        let decimals = 6u8; // USDC-like decimals

        // Create payer
        let payer = Keypair::new();
        svm.airdrop(&payer.pubkey(), LAMPORTS_PER_SOL).unwrap();

        // Derive the expected wrapped mint PDA
        let (expected_mint, _) = Pubkey::find_program_address(
            &[
                WRAPPED_TOKEN_SEED,
                decimals.to_le_bytes().as_ref(),
                partial_token_metadata.hash().as_ref(),
            ],
            &TOKEN_BRIDGE_PROGRAM_ID,
        );

        // Build the wrap_token instruction
        let wrap_token_accounts = crate::accounts::WrapToken {
            payer: payer.pubkey(),
            mint: expected_mint,
            token_program: SPL_TOKEN_PROGRAM_ID,
            system_program: solana_sdk_ids::system_program::ID,
        };

        let wrap_token_ix = Instruction {
            program_id: TOKEN_BRIDGE_PROGRAM_ID,
            accounts: wrap_token_accounts.to_account_metas(None),
            data: crate::instruction::WrapToken {
                decimals,
                partial_token_metadata: partial_token_metadata.clone(),
            }
            .data(),
        };

        // Build and send the transaction
        let tx = Transaction::new(
            &[&payer],
            Message::new(&[wrap_token_ix], Some(&payer.pubkey())),
            svm.latest_blockhash(),
        );

        svm.send_transaction(tx)
            .expect("Transaction should succeed");

        // Verify that the mint was created correctly
        let mint_account = svm.get_account(&expected_mint).unwrap();
        assert_eq!(mint_account.owner, SPL_TOKEN_PROGRAM_ID);

        // Deserialize and verify mint properties
        let mint_data = mint_account.data;
        let mint_with_extension = StateWithExtensions::<Mint>::unpack(&mint_data).unwrap();
        let mint = mint_with_extension.base;

        assert_eq!(mint.decimals, decimals);
        assert_eq!(mint.mint_authority, Some(expected_mint).into());
        assert_eq!(mint.freeze_authority, Some(expected_mint).into());
        assert!(mint.is_initialized);
        assert_eq!(mint.supply, 0);

        // Verify metadata pointer extension
        let metadata_pointer = mint_with_extension
            .get_extension::<MetadataPointer>()
            .unwrap();
        assert_eq!(metadata_pointer.authority, None.try_into().unwrap());
        assert_eq!(
            metadata_pointer.metadata_address,
            Some(expected_mint).try_into().unwrap()
        );

        // Verify token metadata
        let token_metadata = mint_with_extension
            .get_variable_len_extension::<TokenMetadata>()
            .unwrap();
        assert_eq!(token_metadata.name, partial_token_metadata.name);
        assert_eq!(token_metadata.symbol, partial_token_metadata.symbol);

        assert_eq!(token_metadata.additional_metadata.len(), 1);
        let (key, value) = &token_metadata.additional_metadata[0];
        assert_eq!(key, REMOTE_TOKEN_METADATA_KEY);
        assert_eq!(value, &hex::encode(partial_token_metadata.remote_token));
    }
}
