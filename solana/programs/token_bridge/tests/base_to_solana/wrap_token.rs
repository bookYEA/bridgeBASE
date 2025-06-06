use anchor_lang::{prelude::*, solana_program::native_token::LAMPORTS_PER_SOL, InstructionData};
use anchor_spl::{
    token_2022::spl_token_2022::{
        extension::{BaseStateWithExtensions, StateWithExtensions},
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

use token_bridge::{
    constants::{REMOTE_TOKEN_METADATA_KEY, WRAPPED_TOKEN_SEED},
    instructions::PartialTokenMetadata,
    ID as TOKEN_BRIDGE_PROGRAM_ID,
};

use crate::base_to_solana::SPL_TOKEN_PROGRAM_ID;

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
    let wrap_token_accounts = token_bridge::accounts::WrapToken {
        payer: payer.pubkey(),
        mint: expected_mint,
        token_program: SPL_TOKEN_PROGRAM_ID,
        system_program: solana_sdk_ids::system_program::ID,
    };

    let wrap_token_ix = Instruction {
        program_id: TOKEN_BRIDGE_PROGRAM_ID,
        accounts: wrap_token_accounts.to_account_metas(None),
        data: token_bridge::instruction::WrapToken {
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
