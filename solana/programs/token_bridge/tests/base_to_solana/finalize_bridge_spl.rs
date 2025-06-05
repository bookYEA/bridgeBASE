use anchor_lang::{prelude::*, InstructionData};
use anchor_spl::token::spl_token::state::Account as TokenAccount;
use litesvm::LiteSVM;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_message::Message;
use solana_program_pack::Pack;
use solana_signer::Signer;

use portal::{internal::Ix, ID as PORTAL_PROGRAM_ID};
use solana_transaction::Transaction;
use token_bridge::constants::REMOTE_BRIDGE;
use token_bridge::ID as TOKEN_BRIDGE_PROGRAM_ID;

use crate::base_to_solana::mock_remote_call;
use crate::base_to_solana::{
    mock_mint, mock_token_account, mock_token_vault, portal_authority, SPL_TOKEN_PROGRAM_ID,
};

const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

#[test]
fn test_finalize_bridge_spl_success() {
    let mut svm = LiteSVM::new();
    svm.add_program_from_file(
        TOKEN_BRIDGE_PROGRAM_ID,
        "../../target/deploy/token_bridge.so",
    )
    .unwrap();
    svm.add_program_from_file(PORTAL_PROGRAM_ID, "../../target/deploy/portal.so")
        .unwrap();

    // Test parameters
    let remote_token = [0x42u8; 20]; // Sample remote token address
    let decimals = 6u8; // USDC-like decimals
    let bridge_amount = 1000 * 10_u64.pow(decimals as u32); // 1000 tokens
    let vault_initial_balance = 10000 * 10_u64.pow(decimals as u32); // 10000 tokens

    // Create payer
    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), LAMPORTS_PER_SOL).unwrap();

    // Create recipient
    let recipient = Keypair::new();
    let recipient_pk = recipient.pubkey();

    // Create mint
    let mint = Keypair::new().pubkey();
    mock_mint(&mut svm, mint, decimals);

    // Create token vault with funds
    let token_vault = mock_token_vault(&mut svm, mint, remote_token, vault_initial_balance);

    // Create destination token account
    let to_token_account = Keypair::new().pubkey();
    mock_token_account(&mut svm, to_token_account, mint, recipient_pk, 0);

    // Compute the portal authority PDA
    let portal_authority = portal_authority();

    // Build the TokenBridge's finalize_bridge_spl instruction
    let finalize_bridge_spl_accounts = token_bridge::accounts::FinalizeBridgeSpl {
        portal_authority,
        mint,
        token_vault,
        to_token_account,
        token_program: SPL_TOKEN_PROGRAM_ID,
    }
    .to_account_metas(None)
    .into_iter()
    .skip(1) // Skip portal_authority since relay_call handles it
    .collect::<Vec<_>>();

    let finalize_bridge_spl_ix = Ix::from(Instruction {
        program_id: TOKEN_BRIDGE_PROGRAM_ID,
        accounts: finalize_bridge_spl_accounts.clone(),
        data: token_bridge::instruction::FinalizeBridgeSpl {
            remote_token,
            amount: bridge_amount,
        }
        .data(),
    });

    // Build the Portal's relay_call instruction
    let remote_call = mock_remote_call(
        &mut svm,
        REMOTE_BRIDGE,
        vec![finalize_bridge_spl_ix].try_to_vec().unwrap(),
        false,
    );

    let mut relay_call_accounts = portal::accounts::RelayCall {
        portal_authority,
        payer: payer.pubkey(),
        remote_call,
    }
    .to_account_metas(None);

    // Add the finalize_bridge_spl accounts and token program to the relay_call instruction
    relay_call_accounts.extend_from_slice(&finalize_bridge_spl_accounts);
    relay_call_accounts.push(AccountMeta::new_readonly(TOKEN_BRIDGE_PROGRAM_ID, false));

    let relay_call_ix = Instruction {
        program_id: PORTAL_PROGRAM_ID,
        accounts: relay_call_accounts,
        data: portal::instruction::RelayCall {}.data(),
    };

    // Build and send the transaction
    let tx = Transaction::new(
        &[&payer],
        Message::new(&[relay_call_ix], Some(&payer.pubkey())),
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx)
        .expect("Transaction should succeed");

    // Verify that tokens were transferred from vault to recipient
    let to_token_account_after = svm.get_account(&to_token_account).unwrap();
    let to_token_account_after = TokenAccount::unpack(&to_token_account_after.data).unwrap();
    assert_eq!(
        to_token_account_after.amount, bridge_amount,
        "Recipient should receive the bridged tokens"
    );

    let token_vault_after = svm.get_account(&token_vault).unwrap();
    let token_vault_after = TokenAccount::unpack(&token_vault_after.data).unwrap();
    assert_eq!(
        token_vault_after.amount,
        vault_initial_balance - bridge_amount,
        "Vault should have reduced balance"
    );
}
