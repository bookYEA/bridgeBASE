use anchor_lang::{
    prelude::*, solana_program::native_token::LAMPORTS_PER_SOL, AccountDeserialize,
    InstructionData, ToAccountMetas,
};
use litesvm::LiteSVM;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;

use portal::{
    constants::{GAS_FEE_RECEIVER, MESSENGER_SEED},
    state::Messenger,
    ID as PORTAL_PROGRAM_ID,
};

use crate::solana_to_base::mock_messenger;

#[test]
fn test_send_message_fail_uninitialized_messenger() {
    let mut svm = LiteSVM::new();
    svm.add_program_from_file(PORTAL_PROGRAM_ID, "../../target/deploy/portal.so")
        .unwrap();

    // Create test accounts
    let payer = Keypair::new();
    let payer_pk = payer.pubkey();
    svm.airdrop(&payer_pk, LAMPORTS_PER_SOL).unwrap();

    let authority = Keypair::new();
    let authority_pk = authority.pubkey();

    // Don't initialize messenger - this should cause failure
    let (messenger_pda, _) = Pubkey::find_program_address(&[MESSENGER_SEED], &PORTAL_PROGRAM_ID);

    // Test parameters
    let target = [1u8; 20];
    let message = b"test message".to_vec();
    let min_gas_limit = 100_000u64;

    // Build the send_message instruction
    let send_message_accounts = portal::accounts::SendMessage {
        messenger: messenger_pda,
        payer: payer_pk,
        authority: authority_pk,
        gas_fee_receiver: GAS_FEE_RECEIVER,
        system_program: solana_sdk_ids::system_program::ID,
    }
    .to_account_metas(None);

    let send_message_ix = Instruction {
        program_id: PORTAL_PROGRAM_ID,
        accounts: send_message_accounts,
        data: portal::instruction::SendMessage {
            target,
            message,
            min_gas_limit,
        }
        .data(),
    };

    // Build and send the transaction
    let tx = Transaction::new(
        &[&payer, &authority],
        Message::new(&[send_message_ix], Some(&payer_pk)),
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(
        result.is_err(),
        "Transaction should fail with uninitialized messenger"
    );
}

#[test]
fn test_send_message_success() {
    let mut svm = LiteSVM::new();
    svm.add_program_from_file(PORTAL_PROGRAM_ID, "../../target/deploy/portal.so")
        .unwrap();

    // Create test accounts
    let payer = Keypair::new();
    let payer_pk = payer.pubkey();
    svm.airdrop(&payer_pk, LAMPORTS_PER_SOL).unwrap();

    let authority = Keypair::new();
    let authority_pk = authority.pubkey();

    // Mock the messenger account instead of initializing it
    let (messenger_pda, _) = Pubkey::find_program_address(&[MESSENGER_SEED], &PORTAL_PROGRAM_ID);
    let initial_nonce = 42u64;
    mock_messenger(&mut svm, &messenger_pda, initial_nonce);

    // Test parameters
    let target = [0x42u8; 20]; // Sample target address
    let message = b"Hello, Base chain!".to_vec();
    let min_gas_limit = 100_000u64;

    // Build the send_message instruction
    let send_message_accounts = portal::accounts::SendMessage {
        messenger: messenger_pda,
        payer: payer_pk,
        authority: authority_pk,
        gas_fee_receiver: GAS_FEE_RECEIVER,
        system_program: solana_sdk_ids::system_program::ID,
    }
    .to_account_metas(None);

    let send_message_ix = Instruction {
        program_id: PORTAL_PROGRAM_ID,
        accounts: send_message_accounts,
        data: portal::instruction::SendMessage {
            target,
            message: message.clone(),
            min_gas_limit,
        }
        .data(),
    };

    // Build and send the transaction
    let tx = Transaction::new(
        &[&payer, &authority],
        Message::new(&[send_message_ix], Some(&payer_pk)),
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "Transaction should succeed");
    // TODO: Check that the correct event is emitted

    // Verify that the messenger nonce was incremented
    let messenger_account_after = svm.get_account(&messenger_pda).unwrap();
    let messenger_data_after =
        Messenger::try_deserialize(&mut &messenger_account_after.data[..]).unwrap();
    assert_eq!(
        messenger_data_after.nonce,
        initial_nonce + 1,
        "Messenger nonce should be incremented"
    );

    // Verify that gas fee was transferred to the gas fee receiver
    let gas_fee_receiver_account = svm.get_account(&GAS_FEE_RECEIVER).unwrap();
    assert!(
        gas_fee_receiver_account.lamports > 0,
        "Gas fee receiver should have received lamports"
    );
}
