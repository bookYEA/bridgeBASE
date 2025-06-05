pub mod bridge_back_sol;
pub mod bridge_back_spl;
pub mod bridge_token;

use anchor_lang::prelude::*;
use anchor_lang::solana_program::program_option::COption;
use anchor_spl::token::spl_token::state::{Account as TokenAccount, AccountState, Mint};
use litesvm::LiteSVM;
use portal::{constants::PORTAL_AUTHORITY_SEED, state::RemoteCall};
use solana_account::Account;
use solana_keypair::Keypair;
use solana_program_pack::Pack;
use solana_signer::Signer;

use portal::ID as PORTAL_PROGRAM_ID;
use token_bridge::{
    constants::{REMOTE_BRIDGE, SOL_VAULT_SEED, TOKEN_VAULT_SEED, WRAPPED_TOKEN_SEED},
    ID as TOKEN_BRIDGE_PROGRAM_ID,
};

pub const SPL_TOKEN_PROGRAM_ID: Pubkey = anchor_spl::token::ID;

fn portal_authority() -> Pubkey {
    let (portal_authority, _) = Pubkey::find_program_address(
        &[PORTAL_AUTHORITY_SEED, REMOTE_BRIDGE.as_ref()],
        &PORTAL_PROGRAM_ID,
    );

    portal_authority
}

fn mock_remote_call(svm: &mut LiteSVM, sender: [u8; 20], data: Vec<u8>, executed: bool) -> Pubkey {
    let remote_call = Keypair::new().pubkey();

    let mut remote_call_data = Vec::new();
    RemoteCall {
        sender,
        data,
        executed,
    }
    .try_serialize(&mut remote_call_data)
    .unwrap();

    svm.set_account(
        remote_call,
        Account {
            lamports: 0,
            data: remote_call_data,
            owner: PORTAL_PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();

    remote_call
}

fn mock_sol_vault(svm: &mut LiteSVM, remote_token: [u8; 20], lamports: u64) -> Pubkey {
    let (sol_vault, _) = Pubkey::find_program_address(
        &[SOL_VAULT_SEED, remote_token.as_ref()],
        &TOKEN_BRIDGE_PROGRAM_ID,
    );

    svm.set_account(
        sol_vault,
        Account {
            lamports,
            data: Vec::new(),
            owner: solana_sdk_ids::system_program::ID,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();

    sol_vault
}

fn mock_wrapped_mint(svm: &mut LiteSVM, remote_token: [u8; 20], decimals: u8) -> Pubkey {
    let (wrapped_mint, _) = Pubkey::find_program_address(
        &[
            WRAPPED_TOKEN_SEED,
            remote_token.as_ref(),
            decimals.to_le_bytes().as_ref(),
        ],
        &TOKEN_BRIDGE_PROGRAM_ID,
    );

    mock_mint(svm, wrapped_mint, decimals);
    wrapped_mint
}

fn mock_mint(svm: &mut LiteSVM, mint: Pubkey, decimals: u8) {
    let mut mint_data = vec![0u8; 82]; // Mint account size
    Mint {
        mint_authority: COption::Some(mint),
        supply: 1_000_000 * 10_u64.pow(decimals as u32),
        decimals,
        is_initialized: true,
        freeze_authority: COption::None,
    }
    .pack_into_slice(&mut mint_data);

    svm.set_account(
        mint,
        Account {
            lamports: 0,
            data: mint_data,
            owner: SPL_TOKEN_PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();
}

fn mock_token_vault(
    svm: &mut LiteSVM,
    mint: Pubkey,
    remote_token: [u8; 20],
    amount: u64,
) -> Pubkey {
    let (token_vault, _) = Pubkey::find_program_address(
        &[TOKEN_VAULT_SEED, mint.as_ref(), remote_token.as_ref()],
        &TOKEN_BRIDGE_PROGRAM_ID,
    );

    mock_token_account(svm, token_vault, mint, token_vault, amount);
    token_vault
}

fn mock_token_account(
    svm: &mut LiteSVM,
    token_account: Pubkey,
    mint: Pubkey,
    owner: Pubkey,
    amount: u64,
) {
    // Create token account data (SPL Token Account layout)
    let mut token_account_data = vec![0u8; 165]; // Token account size
    TokenAccount {
        mint,
        owner,
        amount,
        delegate: COption::None,
        state: AccountState::Initialized,
        is_native: COption::None,
        delegated_amount: 0,
        close_authority: COption::None,
    }
    .pack_into_slice(&mut token_account_data);

    svm.set_account(
        token_account,
        Account {
            lamports: 0,
            data: token_account_data,
            owner: SPL_TOKEN_PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();
}
