use anchor_lang::prelude::*;

use crate::{
    constants::{OUTPUT_ROOT_SEED, TRUSTED_ORACLE},
    state::OutputRoot,
};

#[derive(Accounts)]
#[instruction(_output_root: [u8; 32], block_number: u64)]
pub struct RegisterOutputRoot<'info> {
    #[account(mut, address = TRUSTED_ORACLE @ RegisterOutputRootError::Unauthorized)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + OutputRoot::INIT_SPACE,
        seeds = [OUTPUT_ROOT_SEED, &block_number.to_le_bytes()],
        bump
    )]
    pub root: Account<'info, OutputRoot>,

    pub system_program: Program<'info, System>,
}

pub fn register_output_root_handler(
    ctx: Context<RegisterOutputRoot>,
    output_root: [u8; 32],
    _block_number: u64,
) -> Result<()> {
    // TODO: Plug some ISM verification here.

    ctx.accounts.root.root = output_root;

    Ok(())
}

#[error_code]
pub enum RegisterOutputRootError {
    #[msg("Unauthorized")]
    Unauthorized,
}

#[cfg(all(test, not(any(feature = "devnet", feature = "mainnet"))))]
mod tests {
    use super::*;

    use anchor_lang::{solana_program::native_token::LAMPORTS_PER_SOL, InstructionData};

    use litesvm::LiteSVM;
    use solana_instruction::Instruction;
    use solana_keypair::Keypair;
    use solana_message::Message;
    use solana_signer::Signer;
    use solana_transaction::Transaction;

    use crate::{constants::TRUSTED_ORACLE_KEYPAIR_BASE58, ID as PORTAL_PROGRAM_ID};

    #[test]
    fn test_register_output_root_fail_unauthorized() {
        let mut svm = LiteSVM::new();
        svm.add_program_from_file(PORTAL_PROGRAM_ID, "../../target/deploy/portal.so")
            .unwrap();

        // Create test accounts - use wrong payer (not TRUSTED_ORACLE)
        let wrong_payer = Keypair::new();
        let wrong_payer_pk = wrong_payer.pubkey();
        svm.airdrop(&wrong_payer_pk, LAMPORTS_PER_SOL).unwrap();

        // Test parameters
        let output_root = [42u8; 32];
        let block_number = 12345u64;

        // Create PDAs
        let (output_root_pda, _) = Pubkey::find_program_address(
            &[OUTPUT_ROOT_SEED, &block_number.to_le_bytes()],
            &PORTAL_PROGRAM_ID,
        );

        // Build the instruction with wrong payer
        let register_output_root_accounts = crate::accounts::RegisterOutputRoot {
            payer: wrong_payer_pk, // This should fail because it's not TRUSTED_ORACLE
            root: output_root_pda,
            system_program: solana_sdk_ids::system_program::ID,
        }
        .to_account_metas(None);

        let register_output_root_ix = Instruction {
            program_id: PORTAL_PROGRAM_ID,
            accounts: register_output_root_accounts,
            data: crate::instruction::RegisterOutputRoot {
                output_root,
                block_number,
            }
            .data(),
        };

        // Build and send the transaction
        let tx = Transaction::new(
            &[&wrong_payer],
            Message::new(&[register_output_root_ix], Some(&wrong_payer_pk)),
            svm.latest_blockhash(),
        );

        let result = svm.send_transaction(tx);
        match result {
            Ok(_) => {
                panic!("Transaction should fail with unauthorized payer");
            }
            Err(e) => {
                let correct_error = e.meta.logs.iter().any(|log| {
                    log.contains(
                        "Program log: AnchorError caused by account: payer. Error Code: Unauthorized",
                    )
                });
                assert!(
                    correct_error,
                    "Transaction should fail with unauthorized payer"
                );
            }
        }
    }

    #[test]
    fn test_register_output_root_success() {
        let mut svm = LiteSVM::new();
        svm.add_program_from_file(PORTAL_PROGRAM_ID, "../../target/deploy/portal.so")
            .unwrap();

        // Create test accounts - use the correct TRUSTED_ORACLE
        let trusted_oracle_keypair = Keypair::from_base58_string(TRUSTED_ORACLE_KEYPAIR_BASE58);
        let trusted_oracle_pubkey = trusted_oracle_keypair.pubkey();

        // Airdrop to trusted oracle
        svm.airdrop(&trusted_oracle_pubkey, LAMPORTS_PER_SOL)
            .unwrap();

        // Test parameters
        let output_root = [42u8; 32];
        let block_number = 12345u64;

        // Create PDAs
        let (output_root_pda, _) = Pubkey::find_program_address(
            &[OUTPUT_ROOT_SEED, &block_number.to_le_bytes()],
            &PORTAL_PROGRAM_ID,
        );

        // Build the instruction
        let register_output_root_accounts = crate::accounts::RegisterOutputRoot {
            payer: trusted_oracle_pubkey,
            root: output_root_pda,
            system_program: solana_sdk_ids::system_program::ID,
        }
        .to_account_metas(None);

        let register_output_root_ix = Instruction {
            program_id: PORTAL_PROGRAM_ID,
            accounts: register_output_root_accounts,
            data: crate::instruction::RegisterOutputRoot {
                output_root,
                block_number,
            }
            .data(),
        };

        // Build and send the transaction
        let tx = Transaction::new(
            &[&trusted_oracle_keypair],
            Message::new(
                &[register_output_root_ix],
                Some(&trusted_oracle_keypair.pubkey()),
            ),
            svm.latest_blockhash(),
        );

        let result = svm.send_transaction(tx);
        assert!(result.is_ok(), "Transaction should succeed");

        // Verify that the output root was correctly registered
        let output_root_account = svm.get_account(&output_root_pda).unwrap();
        let output_root_data =
            OutputRoot::try_deserialize(&mut &output_root_account.data[..]).unwrap();
        assert_eq!(
            output_root_data.root, output_root,
            "Output root should be correctly stored"
        );
    }
}
