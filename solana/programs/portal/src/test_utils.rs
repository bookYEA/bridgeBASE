#[cfg(test)]
use anchor_lang::{prelude::*, solana_program::native_token::LAMPORTS_PER_SOL};
use litesvm::LiteSVM;
use solana_account::Account;

use crate::{state::Messenger, ID as PORTAL_PROGRAM_ID};

pub fn mock_messenger(svm: &mut LiteSVM, messenger_pda: &Pubkey, nonce: u64) {
    let mut messenger_data = Vec::new();
    Messenger { nonce }
        .try_serialize(&mut messenger_data)
        .unwrap();

    svm.set_account(
        *messenger_pda,
        Account {
            lamports: LAMPORTS_PER_SOL, // Rent-exempt amount
            data: messenger_data,
            owner: PORTAL_PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();
}
