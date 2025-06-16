use anchor_lang::prelude::*;

use crate::{
    constants::{EIP1559_SEED, MESSENGER_SEED},
    state::{Eip1559, Messenger},
};

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

    #[account(
        init,
        payer = payer,
        seeds = [EIP1559_SEED],
        bump,
        space = 8 + Eip1559::INIT_SPACE
    )]
    pub eip1559: Account<'info, Eip1559>,

    pub system_program: Program<'info, System>,
}

pub fn initialize_handler(ctx: Context<Initialize>) -> Result<()> {
    let current_timestamp = Clock::get()?.unix_timestamp;
    *ctx.accounts.eip1559 = Eip1559::new(current_timestamp);

    Ok(())
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

    use crate::{
        constants::{
            EIP1559_DEFAULT_ADJUSTMENT_DENOMINATOR, EIP1559_DEFAULT_GAS_TARGET_PER_WINDOW,
            EIP1559_INITIAL_BASE_FEE_GWEI,
        },
        test_utils::mock_clock,
        ID as PORTAL_PROGRAM_ID,
    };

    #[test]
    fn test_initialize_success() {
        let mut svm = LiteSVM::new();
        svm.add_program_from_file(PORTAL_PROGRAM_ID, "../../target/deploy/portal.so")
            .unwrap();

        // Create test accounts
        let payer = Keypair::new();
        let payer_pk = payer.pubkey();
        svm.airdrop(&payer_pk, LAMPORTS_PER_SOL).unwrap();

        // Mock the clock to ensure we get a proper timestamp
        mock_clock(&mut svm, 1747440000); // May 16th, 2025

        // Find the PDAs
        let (messenger, _) = Pubkey::find_program_address(&[MESSENGER_SEED], &PORTAL_PROGRAM_ID);
        let (eip1559, _) = Pubkey::find_program_address(&[EIP1559_SEED], &PORTAL_PROGRAM_ID);

        // Build the instruction
        let initialize_accounts = crate::accounts::Initialize {
            payer: payer_pk,
            messenger,
            eip1559,
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

        // Assert the expected messenger account data
        let messenger_account = svm.get_account(&messenger).unwrap();
        assert_eq!(messenger_account.owner, PORTAL_PROGRAM_ID);

        let messenger_data = Messenger::try_deserialize(&mut &messenger_account.data[..]).unwrap();
        assert_eq!(messenger_data.nonce, 0);

        // Assert the expected Eip1559 account data
        let eip1559_account = svm.get_account(&eip1559).unwrap();
        assert_eq!(eip1559_account.owner, PORTAL_PROGRAM_ID);

        let eip1559_data = Eip1559::try_deserialize(&mut &eip1559_account.data[..]).unwrap();
        assert_eq!(eip1559_data.target, EIP1559_DEFAULT_GAS_TARGET_PER_WINDOW);
        assert_eq!(
            eip1559_data.denominator,
            EIP1559_DEFAULT_ADJUSTMENT_DENOMINATOR
        );
        assert_eq!(eip1559_data.current_base_fee, EIP1559_INITIAL_BASE_FEE_GWEI);
        assert_eq!(eip1559_data.current_window_gas_used, 0);
        assert_eq!(eip1559_data.window_start_time, 1747440000);
    }
}
