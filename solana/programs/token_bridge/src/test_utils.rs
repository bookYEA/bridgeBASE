#[cfg(test)]
use anchor_lang::solana_program::program_option::COption;
use anchor_lang::{prelude::*, solana_program::native_token::LAMPORTS_PER_SOL};
use anchor_spl::{
    token_2022::spl_token_2022::{
        extension::{
            metadata_pointer::MetadataPointer, BaseStateWithExtensionsMut, ExtensionType,
            StateWithExtensionsMut,
        },
        state::{Account as TokenAccount, AccountState, Mint},
    },
    token_interface::spl_token_metadata_interface::state::TokenMetadata,
};
use litesvm::LiteSVM;
use portal::{
    constants::{EIP1559_SEED, PORTAL_AUTHORITY_SEED},
    state::{Eip1559, RemoteCall},
};
use solana_account::Account;
use solana_keypair::Keypair;
use solana_program_pack::Pack;
use solana_signer::Signer;

use crate::{
    constants::{
        BRIDGE_AUTHORITY_SEED, REMOTE_BRIDGE, SOL_VAULT_SEED, TOKEN_VAULT_SEED, WRAPPED_TOKEN_SEED,
    },
    internal::metadata::PartialTokenMetadata,
    ID as TOKEN_BRIDGE_PROGRAM_ID,
};
use portal::ID as PORTAL_PROGRAM_ID;

pub fn portal_authority() -> Pubkey {
    let (portal_authority, _) = Pubkey::find_program_address(
        &[PORTAL_AUTHORITY_SEED, REMOTE_BRIDGE.as_ref()],
        &PORTAL_PROGRAM_ID,
    );

    portal_authority
}

pub fn mock_clock(svm: &mut LiteSVM, timestamp: i64) {
    let mut clock = svm.get_sysvar::<Clock>();
    clock.unix_timestamp = timestamp;
    svm.set_sysvar::<Clock>(&clock);
}

pub fn mock_eip1559(svm: &mut LiteSVM, eip1559: Eip1559) -> Pubkey {
    let (eip1559_pda, _) = Pubkey::find_program_address(&[EIP1559_SEED], &PORTAL_PROGRAM_ID);

    let mut eip1559_data = Vec::new();
    eip1559.try_serialize(&mut eip1559_data).unwrap();

    svm.set_account(
        eip1559_pda,
        Account {
            lamports: 0,
            data: eip1559_data,
            owner: PORTAL_PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();

    eip1559_pda
}

pub fn bridge_authority() -> Pubkey {
    let (bridge_authority, _) =
        Pubkey::find_program_address(&[BRIDGE_AUTHORITY_SEED], &TOKEN_BRIDGE_PROGRAM_ID);
    bridge_authority
}

pub fn mock_remote_call(
    svm: &mut LiteSVM,
    sender: [u8; 20],
    data: Vec<u8>,
    executed: bool,
) -> Pubkey {
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

pub fn mock_sol_vault(svm: &mut LiteSVM, remote_token: [u8; 20], lamports: u64) -> Pubkey {
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

pub fn mock_wrapped_mint(
    svm: &mut LiteSVM,
    decimals: u8,
    partial_token_metadata: &PartialTokenMetadata,
) -> Pubkey {
    let (wrapped_mint, _) = Pubkey::find_program_address(
        &[
            WRAPPED_TOKEN_SEED,
            decimals.to_le_bytes().as_ref(),
            partial_token_metadata.hash().as_ref(),
        ],
        &TOKEN_BRIDGE_PROGRAM_ID,
    );

    let token_metadata = TokenMetadata::from(partial_token_metadata);

    let mut account_size =
        ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::MetadataPointer])
            .unwrap();

    println!("account_size: {:?}", account_size);

    account_size += token_metadata.tlv_size_of().unwrap();

    println!("account_size: {:?}", account_size);

    // Full buffer for the mint account
    let mut mint_data = vec![0u8; account_size];

    let mut mint_with_extension =
        StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut mint_data[..]).unwrap();

    // Initialize the metadata pointer extension
    let metadata_pointer = mint_with_extension
        .init_extension::<MetadataPointer>(false)
        .unwrap();

    metadata_pointer.authority = Some(wrapped_mint).try_into().unwrap();
    metadata_pointer.metadata_address = Some(wrapped_mint).try_into().unwrap();

    // Initialize the token metadata extension
    mint_with_extension
        .init_variable_len_extension(&token_metadata, false)
        .unwrap();

    // Initialize the mint account
    mint_with_extension.base = Mint {
        mint_authority: COption::Some(wrapped_mint),
        supply: 0,
        decimals,
        is_initialized: true,
        freeze_authority: COption::None,
    };
    mint_with_extension.pack_base();
    mint_with_extension.init_account_type().unwrap();

    svm.set_account(
        wrapped_mint,
        Account {
            lamports: 100 * LAMPORTS_PER_SOL,
            data: mint_data,
            owner: anchor_spl::token_2022::ID,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();

    wrapped_mint
}

pub fn mock_mint(svm: &mut LiteSVM, mint: Pubkey, decimals: u8, token_program_id: Pubkey) {
    let mut mint_data = vec![0u8; Mint::LEN];
    Mint {
        mint_authority: COption::Some(mint),
        supply: 0,
        decimals,
        is_initialized: true,
        freeze_authority: COption::None,
    }
    .pack_into_slice(&mut mint_data);

    svm.set_account(
        mint,
        Account {
            lamports: 100 * LAMPORTS_PER_SOL,
            data: mint_data,
            owner: token_program_id,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();
}

pub fn mock_token_vault(
    svm: &mut LiteSVM,
    mint: Pubkey,
    remote_token: [u8; 20],
    amount: u64,
    token_program_id: Pubkey,
) -> Pubkey {
    let (token_vault, _) = Pubkey::find_program_address(
        &[TOKEN_VAULT_SEED, mint.as_ref(), remote_token.as_ref()],
        &TOKEN_BRIDGE_PROGRAM_ID,
    );

    mock_token_account(
        svm,
        token_vault,
        mint,
        token_vault,
        amount,
        token_program_id,
    );
    token_vault
}

pub fn mock_token_account(
    svm: &mut LiteSVM,
    token_account: Pubkey,
    mint: Pubkey,
    owner: Pubkey,
    amount: u64,
    token_program_id: Pubkey,
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
            lamports: 100 * LAMPORTS_PER_SOL,
            data: token_account_data,
            owner: token_program_id,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();
}
