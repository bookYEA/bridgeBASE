use anchor_lang::prelude::*;
use litesvm::LiteSVM;
use solana_account::Account;

use crate::{
    constants::{EIP1559_SEED, MESSENGER_SEED},
    state::{Eip1559, Messenger},
    ID as PORTAL_PROGRAM_ID,
};

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

pub fn mock_messenger(svm: &mut LiteSVM, nonce: u64) -> Pubkey {
    let (messenger_pda, _) = Pubkey::find_program_address(&[MESSENGER_SEED], &PORTAL_PROGRAM_ID);

    let mut messenger_data = Vec::new();
    Messenger { nonce }
        .try_serialize(&mut messenger_data)
        .unwrap();

    svm.set_account(
        messenger_pda,
        Account {
            lamports: 0,
            data: messenger_data,
            owner: PORTAL_PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();

    messenger_pda
}
