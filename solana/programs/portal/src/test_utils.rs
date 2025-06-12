use anchor_lang::{prelude::*, solana_program::native_token::LAMPORTS_PER_SOL};
use litesvm::LiteSVM;
use solana_account::Account;

use crate::{constants::MESSENGER_SEED, state::Messenger, ID as PORTAL_PROGRAM_ID};

pub fn mock_messenger(svm: &mut LiteSVM, nonce: u64) -> Pubkey {
    let (messenger_pda, _) = Pubkey::find_program_address(&[MESSENGER_SEED], &PORTAL_PROGRAM_ID);

    let mut messenger_data = Vec::new();
    Messenger { nonce }
        .try_serialize(&mut messenger_data)
        .unwrap();

    svm.set_account(
        messenger_pda,
        Account {
            lamports: LAMPORTS_PER_SOL, // Rent-exempt amount
            data: messenger_data,
            owner: PORTAL_PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();

    messenger_pda
}
