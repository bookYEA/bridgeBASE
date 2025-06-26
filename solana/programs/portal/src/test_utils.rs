use anchor_lang::prelude::*;
use litesvm::LiteSVM;
use solana_account::Account;

use crate::{
    constants::{CALL_SEED, PORTAL_SEED},
    state::Portal,
    ID as PORTAL_PROGRAM_ID,
};

pub fn mock_clock(svm: &mut LiteSVM, timestamp: i64) {
    let mut clock = svm.get_sysvar::<Clock>();
    clock.unix_timestamp = timestamp;
    svm.set_sysvar::<Clock>(&clock);
}

pub fn mock_portal(svm: &mut LiteSVM, portal: Portal) -> Pubkey {
    let (portal_pda, _) = Pubkey::find_program_address(&[PORTAL_SEED], &PORTAL_PROGRAM_ID);

    let mut portal_data = Vec::new();
    portal.try_serialize(&mut portal_data).unwrap();

    svm.set_account(
        portal_pda,
        Account {
            lamports: 0,
            data: portal_data,
            owner: PORTAL_PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();

    portal_pda
}

pub fn call_pda(nonce: u64) -> Pubkey {
    let (call_pda, _) = Pubkey::find_program_address(
        &[CALL_SEED, nonce.to_le_bytes().as_ref()],
        &PORTAL_PROGRAM_ID,
    );
    call_pda
}
