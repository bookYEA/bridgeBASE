use anchor_lang::prelude::*;

use crate::{constants::MESSENGER_SEED, state::Messenger};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        seeds = [MESSENGER_SEED],
        bump,
        space = 8 + Messenger::INIT_SPACE
    )]
    pub messenger: Account<'info, Messenger>,

    pub system_program: Program<'info, System>,
}

#[cfg(test)]
mod tests {
    use super::*;

    use anchor_lang::{solana_program::native_token::LAMPORTS_PER_SOL, InstructionData};
    use litesvm::LiteSVM;
    use solana_instruction::Instruction;
    use solana_keypair::Keypair;
    use solana_message::Message;
    use solana_signer::Signer;
    use solana_transaction::Transaction;

    use crate::ID as PORTAL_PROGRAM_ID;

    #[test]
    fn test_initialize_success() {
        let mut svm = LiteSVM::new();
        svm.add_program_from_file(PORTAL_PROGRAM_ID, "../../target/deploy/portal.so")
            .unwrap();

        // Create test accounts
        let payer = Keypair::new();
        let payer_pk = payer.pubkey();
        svm.airdrop(&payer_pk, LAMPORTS_PER_SOL).unwrap();

        // Find the messenger PDA
        let (messenger, _) = Pubkey::find_program_address(&[MESSENGER_SEED], &PORTAL_PROGRAM_ID);

        // Build the instruction
        let initialize_accounts = crate::accounts::Initialize {
            payer: payer_pk,
            messenger,
            system_program: solana_sdk_ids::system_program::ID,
        }
        .to_account_metas(None);

        let initialize_ix = Instruction {
            program_id: PORTAL_PROGRAM_ID,
            accounts: initialize_accounts,
            data: crate::instruction::Initialize {}.data(),
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
        assert_eq!(account.owner, PORTAL_PROGRAM_ID);

        let messenger_account = Messenger::try_deserialize(&mut &account.data[..]).unwrap();
        assert_eq!(messenger_account.nonce, 0);
    }
}
