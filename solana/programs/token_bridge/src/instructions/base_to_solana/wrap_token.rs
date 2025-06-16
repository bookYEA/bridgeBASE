use alloy_primitives::FixedBytes;
use alloy_sol_types::SolCall;
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
use portal::{cpi as portal_cpi, program::Portal};

use crate::constants::{
    BRIDGE_AUTHORITY_SEED, REMOTE_TOKEN_METADATA_KEY, SCALER_EXPONENT_METADATA_KEY,
    WRAPPED_TOKEN_SEED,
};
use crate::internal::{cpi_send_message, metadata::PartialTokenMetadata};
use crate::solidity::Bridge;

#[derive(Accounts)]
#[instruction(decimals: u8, metadata: PartialTokenMetadata)]
pub struct WrapToken<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        // NOTE: Suboptimal to compute the seeds here but it allows to use `init`.
        seeds = [
            WRAPPED_TOKEN_SEED,
            decimals.to_le_bytes().as_ref(),
            metadata.hash().as_ref(),
        ],
        bump,
        mint::decimals = decimals,
        mint::authority = mint,
        // mint::freeze_authority = mint,
        // extensions::metadata_pointer::authority = mint,
        extensions::metadata_pointer::metadata_address = mint,
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    pub token_program: Program<'info, Token2022>,

    pub portal: Program<'info, Portal>,

    // Portal remaining accounts
    /// CHECK: Checked by the Portal program that we CPI into.
    #[account(mut)]
    pub messenger: AccountInfo<'info>,

    /// CHECK: This is the Bridge authority account.
    ///        It is used as the `authority` account when CPIing to the Portal program.
    #[account(seeds = [BRIDGE_AUTHORITY_SEED], bump)]
    pub bridge_authority: AccountInfo<'info>,

    /// CHECK: Checked by the Portal program that we CPI into.
    #[account(mut)]
    pub gas_fee_receiver: AccountInfo<'info>,

    /// CHECK: Checked by the Portal program that we CPI into.
    #[account(mut)]
    pub eip1559: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

pub fn wrap_token_handler(
    ctx: Context<WrapToken>,
    decimals: u8,
    partial_token_metadata: PartialTokenMetadata,
    min_gas_limit: u64,
) -> Result<()> {
    initialize_metadata(&ctx, decimals, &partial_token_metadata)?;

    register_remote_token(
        &ctx,
        &partial_token_metadata.remote_token,
        partial_token_metadata.scaler_exponent,
        min_gas_limit,
    )?;

    Ok(())
}

fn initialize_metadata(
    ctx: &Context<WrapToken>,
    decimals: u8,
    partial_token_metadata: &PartialTokenMetadata,
) -> Result<()> {
    let token_metadata = TokenMetadata::from(partial_token_metadata);

    // FIXME: Computation is most likely unaccurate
    // Calculate lamports required for the additional metadata
    let data_len = token_metadata.tlv_size_of()?;
    let lamports =
        data_len as u64 * DEFAULT_LAMPORTS_PER_BYTE_YEAR * DEFAULT_EXEMPTION_THRESHOLD as u64;

    // Transfer additional lamports to mint account (because we're increasing its size to store the metadata)
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
        &decimals_bytes,
        &metadata_hash,
        &[ctx.bumps.mint],
    ];

    // Initialize token metadata (name, symbol, etc.)
    token_metadata_initialize(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TokenMetadataInitialize {
                program_id: ctx.accounts.token_program.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                metadata: ctx.accounts.mint.to_account_info(),
                mint_authority: ctx.accounts.mint.to_account_info(),
                update_authority: ctx.accounts.mint.to_account_info(),
            },
            &[seeds],
        ),
        token_metadata.name,
        token_metadata.symbol,
        Default::default(),
    )?;

    // Set the remote token metadata key (remote token address)
    token_metadata_update_field(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TokenMetadataUpdateField {
                program_id: ctx.accounts.token_program.to_account_info(),
                metadata: ctx.accounts.mint.to_account_info(),
                update_authority: ctx.accounts.mint.to_account_info(),
            },
            &[seeds],
        ),
        Field::Key(REMOTE_TOKEN_METADATA_KEY.to_string()),
        hex::encode(partial_token_metadata.remote_token),
    )?;

    // Set the scaler exponent metadata key
    token_metadata_update_field(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TokenMetadataUpdateField {
                program_id: ctx.accounts.token_program.to_account_info(),
                metadata: ctx.accounts.mint.to_account_info(),
                update_authority: ctx.accounts.mint.to_account_info(),
            },
            &[seeds],
        ),
        Field::Key(SCALER_EXPONENT_METADATA_KEY.to_string()),
        partial_token_metadata.scaler_exponent.to_string(),
    )?;

    Ok(())
}

fn register_remote_token(
    ctx: &Context<WrapToken>,
    remote_token: &[u8; 20],
    scaler_exponent: u8,
    min_gas_limit: u64,
) -> Result<()> {
    cpi_send_message(
        &ctx.accounts.portal,
        portal_cpi::accounts::SendMessage {
            payer: ctx.accounts.payer.to_account_info(),
            authority: ctx.accounts.bridge_authority.to_account_info(),
            gas_fee_receiver: ctx.accounts.gas_fee_receiver.to_account_info(),
            eip1559: ctx.accounts.eip1559.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            messenger: ctx.accounts.messenger.to_account_info(),
        },
        ctx.bumps.bridge_authority,
        Bridge::registerRemoteTokenCall {
            localToken: remote_token.into(), // NOTE: Intentionally flip the tokens so that when executing on Base it's correct.
            remoteToken: FixedBytes::from(ctx.accounts.mint.key().to_bytes()), // NOTE: Intentionally flip the tokens so that when executing on Base it's correct.
            scalerExponent: scaler_exponent,
        }
        .abi_encode(),
        min_gas_limit,
    )?;

    Ok(())
}

#[error_code]
pub enum WrapTokenError {
    #[msg("Incorrect mint account")]
    IncorrectMintAccount,
}

#[cfg(test)]
mod tests {
    use super::*;

    use anchor_lang::{solana_program::native_token::LAMPORTS_PER_SOL, InstructionData};
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
        test_utils::{bridge_authority, mock_clock, mock_eip1559, mock_messenger},
        ID as TOKEN_BRIDGE_PROGRAM_ID,
    };

    use portal::{constants::GAS_FEE_RECEIVER, state::Eip1559, ID as PORTAL_PROGRAM_ID};

    #[test]
    fn test_wrap_token_success() {
        let mut svm = LiteSVM::new();
        svm.add_program_from_file(
            TOKEN_BRIDGE_PROGRAM_ID,
            "../../target/deploy/token_bridge.so",
        )
        .unwrap();
        svm.add_program_from_file(PORTAL_PROGRAM_ID, "../../target/deploy/portal.so")
            .unwrap();

        println!(
            "token_2022: {:?}",
            hex::encode(anchor_spl::token_2022::ID.to_bytes())
        );
        println!("token: {:?}", hex::encode(anchor_spl::token::ID.to_bytes()));

        // Test parameters
        let partial_token_metadata = PartialTokenMetadata {
            remote_token: [0x42u8; 20],
            name: "Wrapped USDC".to_string(),
            symbol: "WUSDC".to_string(),
            scaler_exponent: 9u8,
        };
        let decimals = 6u8; // USDC-like decimals
        let min_gas_limit = 100_000u64;

        // Create payer
        let payer = Keypair::new();
        svm.airdrop(&payer.pubkey(), LAMPORTS_PER_SOL).unwrap();

        // Derive the expected wrapped mint PDA
        let (wrapped_mint, _) = Pubkey::find_program_address(
            &[
                WRAPPED_TOKEN_SEED,
                decimals.to_le_bytes().as_ref(),
                partial_token_metadata.hash().as_ref(),
            ],
            &TOKEN_BRIDGE_PROGRAM_ID,
        );

        let messenger_pda = mock_messenger(&mut svm, 0);
        let initial_timestamp = 1000i64;
        let eip1559_pda = mock_eip1559(&mut svm, Eip1559::new(initial_timestamp));
        mock_clock(&mut svm, initial_timestamp);

        // Build the wrap_token instruction
        let wrap_token_accounts = crate::accounts::WrapToken {
            payer: payer.pubkey(),
            mint: wrapped_mint,
            token_program: anchor_spl::token_2022::ID,
            bridge_authority: bridge_authority(),
            portal: PORTAL_PROGRAM_ID,
            gas_fee_receiver: GAS_FEE_RECEIVER,
            eip1559: eip1559_pda,
            messenger: messenger_pda,
            system_program: solana_sdk_ids::system_program::ID,
        };

        let wrap_token_ix = Instruction {
            program_id: TOKEN_BRIDGE_PROGRAM_ID,
            accounts: wrap_token_accounts.to_account_metas(None),
            data: crate::instruction::WrapToken {
                decimals,
                partial_token_metadata: partial_token_metadata.clone(),
                min_gas_limit,
            }
            .data(),
        };

        // Build and send the transaction
        let tx = Transaction::new(
            &[&payer],
            Message::new(&[wrap_token_ix], Some(&payer.pubkey())),
            svm.latest_blockhash(),
        );

        let _ = svm
            .send_transaction(tx)
            .expect("Transaction should succeed");

        // Verify that the mint was created correctly
        let mint_account = svm.get_account(&wrapped_mint).unwrap();
        assert_eq!(mint_account.owner, anchor_spl::token_2022::ID);

        // Deserialize and verify mint properties
        let mint_data = mint_account.data;
        let mint_with_extension = StateWithExtensions::<Mint>::unpack(&mint_data).unwrap();
        let mint = mint_with_extension.base;

        assert_eq!(mint.decimals, decimals);
        assert_eq!(mint.mint_authority, Some(wrapped_mint).into());
        assert_eq!(mint.freeze_authority, None.into());
        assert!(mint.is_initialized);
        assert_eq!(mint.supply, 0);

        // Verify metadata pointer extension
        let metadata_pointer = mint_with_extension
            .get_extension::<MetadataPointer>()
            .unwrap();
        assert_eq!(metadata_pointer.authority, None.try_into().unwrap());
        assert_eq!(
            metadata_pointer.metadata_address,
            Some(wrapped_mint).try_into().unwrap()
        );

        // Verify token metadata
        let token_metadata = mint_with_extension
            .get_variable_len_extension::<TokenMetadata>()
            .unwrap();
        assert_eq!(token_metadata.name, partial_token_metadata.name);
        assert_eq!(token_metadata.symbol, partial_token_metadata.symbol);

        assert_eq!(token_metadata.additional_metadata.len(), 2);
        let (key, value) = &token_metadata.additional_metadata[0];
        assert_eq!(key, REMOTE_TOKEN_METADATA_KEY);
        assert_eq!(value, &hex::encode(partial_token_metadata.remote_token));

        let (key, value) = &token_metadata.additional_metadata[1];
        assert_eq!(key, SCALER_EXPONENT_METADATA_KEY);
        assert_eq!(value, &partial_token_metadata.scaler_exponent.to_string());

        // TODO: Verify that a message was sent to register the created SPL to the remote bridge on Base with
        //       the correct scaler exponent.
    }
}
