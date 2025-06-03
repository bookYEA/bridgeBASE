use anchor_lang::prelude::*;
use anchor_lang::InstructionData;
use anchor_lang::ToAccountMetas;
use litesvm::LiteSVM;
use portal::state::Messenger;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;

use portal::{constants::MESSENGER_SEED, ID as PROGRAM_ID};
use solana_transaction::Transaction;

const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

#[test]
fn test_initialize() {
    let mut svm = LiteSVM::new();
    svm.add_program_from_file(PROGRAM_ID, "../../target/deploy/portal.so")
        .unwrap();

    // Create test accounts
    let payer = Keypair::new();
    let payer_pk = payer.pubkey();
    svm.airdrop(&payer_pk, LAMPORTS_PER_SOL).unwrap();

    // Find the messenger PDA
    let (messenger, _) = Pubkey::find_program_address(&[MESSENGER_SEED], &PROGRAM_ID);

    // Build the instruction
    let initialize_accounts = portal::accounts::Initialize {
        payer: payer_pk,
        messenger,
        system_program: solana_sdk_ids::system_program::ID,
    }
    .to_account_metas(None);

    let initialize_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: initialize_accounts,
        data: portal::instruction::Initialize {}.data(),
    };

    // Build and send the transaction
    let tx = Transaction::new(
        &[&payer],
        Message::new(&[initialize_ix], Some(&payer_pk)),
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "Transaction should succeed: {:?}", result);

    // Assert the expected account data
    let account = svm.get_account(&messenger).unwrap();
    assert_eq!(account.owner, PROGRAM_ID);

    let messenger_account = Messenger::try_deserialize(&mut &account.data[..]).unwrap();
    assert_eq!(messenger_account.nonce, 0);
}
