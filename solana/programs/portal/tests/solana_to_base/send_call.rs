use anchor_lang::{
    solana_program::native_token::LAMPORTS_PER_SOL, InstructionData, ToAccountMetas,
};
use litesvm::LiteSVM;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;

use portal::{constants::GAS_FEE_RECEIVER, ID as PORTAL_PROGRAM_ID};

#[test]
fn test_send_call_fail_wrong_gas_fee_receiver() {
    let mut svm = LiteSVM::new();
    svm.add_program_from_file(PORTAL_PROGRAM_ID, "../../target/deploy/portal.so")
        .unwrap();

    // Create test accounts
    let payer = Keypair::new();
    let payer_pk = payer.pubkey();
    svm.airdrop(&payer_pk, LAMPORTS_PER_SOL).unwrap();

    let authority = Keypair::new();
    let authority_pk = authority.pubkey();

    // Use wrong gas fee receiver (not the expected GAS_FEE_RECEIVER)
    let wrong_gas_fee_receiver = Keypair::new().pubkey();

    // Test parameters
    let to = [1u8; 20];
    let gas_limit = 100_000u64;
    let is_creation = false;
    let data = b"hello world".to_vec();

    // Build the instruction with wrong gas fee receiver
    let send_call_accounts = portal::accounts::SendCall {
        payer: payer_pk,
        authority: authority_pk,
        gas_fee_receiver: wrong_gas_fee_receiver, // This should fail
        system_program: solana_sdk_ids::system_program::ID,
    }
    .to_account_metas(None);

    let send_call_ix = Instruction {
        program_id: PORTAL_PROGRAM_ID,
        accounts: send_call_accounts,
        data: portal::instruction::SendCall {
            to,
            gas_limit,
            is_creation,
            data,
        }
        .data(),
    };

    // Build and send the transaction
    let tx = Transaction::new(
        &[&payer, &authority],
        Message::new(&[send_call_ix], Some(&payer_pk)),
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(
        result.is_err(),
        "Transaction should fail with wrong gas fee receiver"
    );
}

#[test]
fn test_send_call_fail_creation_with_non_null_target() {
    let mut svm = LiteSVM::new();
    svm.add_program_from_file(PORTAL_PROGRAM_ID, "../../target/deploy/portal.so")
        .unwrap();

    // Create test accounts
    let payer = Keypair::new();
    let payer_pk = payer.pubkey();
    svm.airdrop(&payer_pk, LAMPORTS_PER_SOL).unwrap();

    let authority = Keypair::new();
    let authority_pk = authority.pubkey();

    // Test parameters - creation call with non-null target (should fail)
    let to = [1u8; 20]; // Non-null address
    let gas_limit = 100_000u64;
    let is_creation = true; // This should require null address
    let data = b"hello world".to_vec();

    // Build the instruction
    let send_call_accounts = portal::accounts::SendCall {
        payer: payer_pk,
        authority: authority_pk,
        gas_fee_receiver: GAS_FEE_RECEIVER,
        system_program: solana_sdk_ids::system_program::ID,
    }
    .to_account_metas(None);

    let send_call_ix = Instruction {
        program_id: PORTAL_PROGRAM_ID,
        accounts: send_call_accounts,
        data: portal::instruction::SendCall {
            to,
            gas_limit,
            is_creation,
            data,
        }
        .data(),
    };

    // Build and send the transaction
    let tx = Transaction::new(
        &[&payer, &authority],
        Message::new(&[send_call_ix], Some(&payer_pk)),
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(
        result.is_err(),
        "Transaction should fail when is_creation=true but target is not null address"
    );
}

#[test]
fn test_send_call_fail_gas_limit_too_low() {
    let mut svm = LiteSVM::new();
    svm.add_program_from_file(PORTAL_PROGRAM_ID, "../../target/deploy/portal.so")
        .unwrap();

    // Create test accounts
    let payer = Keypair::new();
    let payer_pk = payer.pubkey();
    svm.airdrop(&payer_pk, LAMPORTS_PER_SOL).unwrap();

    let authority = Keypair::new();
    let authority_pk = authority.pubkey();

    // Test parameters - very low gas limit
    let to = [1u8; 20];
    let gas_limit = 1u64; // Extremely low gas limit that should fail
    let is_creation = false;
    let data = b"this is a longer message that will require more gas".to_vec();

    // Build the instruction
    let send_call_accounts = portal::accounts::SendCall {
        payer: payer_pk,
        authority: authority_pk,
        gas_fee_receiver: GAS_FEE_RECEIVER,
        system_program: solana_sdk_ids::system_program::ID,
    }
    .to_account_metas(None);

    let send_call_ix = Instruction {
        program_id: PORTAL_PROGRAM_ID,
        accounts: send_call_accounts,
        data: portal::instruction::SendCall {
            to,
            gas_limit,
            is_creation,
            data,
        }
        .data(),
    };

    // Build and send the transaction
    let tx = Transaction::new(
        &[&payer, &authority],
        Message::new(&[send_call_ix], Some(&payer_pk)),
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(
        result.is_err(),
        "Transaction should fail when gas limit is too low"
    );
}

#[test]
fn test_send_call_success() {
    let mut svm = LiteSVM::new();
    svm.add_program_from_file(PORTAL_PROGRAM_ID, "../../target/deploy/portal.so")
        .unwrap();

    // Create test accounts
    let payer = Keypair::new();
    let payer_pk = payer.pubkey();
    svm.airdrop(&payer_pk, LAMPORTS_PER_SOL).unwrap();

    let authority = Keypair::new();
    let authority_pk = authority.pubkey();

    // Test parameters
    let to = [1u8; 20]; // Sample target address
    let gas_limit = 100_000u64;
    let is_creation = false;
    let data = b"hello world".to_vec();

    // Build the instruction
    let send_call_accounts = portal::accounts::SendCall {
        payer: payer_pk,
        authority: authority_pk,
        gas_fee_receiver: GAS_FEE_RECEIVER,
        system_program: solana_sdk_ids::system_program::ID,
    }
    .to_account_metas(None);

    let send_call_ix = Instruction {
        program_id: PORTAL_PROGRAM_ID,
        accounts: send_call_accounts,
        data: portal::instruction::SendCall {
            to,
            gas_limit,
            is_creation,
            data: data.clone(),
        }
        .data(),
    };

    // Build and send the transaction
    let tx = Transaction::new(
        &[&payer, &authority],
        Message::new(&[send_call_ix], Some(&payer_pk)),
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "Transaction should succeed");
    // TODO: Check that the correct event is emitted

    // Verify that gas fee was transferred to the gas fee receiver
    let gas_fee_receiver_account = svm.get_account(&GAS_FEE_RECEIVER).unwrap();
    assert!(
        gas_fee_receiver_account.lamports > 0,
        "Gas fee receiver should have received lamports"
    );
}
