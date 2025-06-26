use anchor_lang::prelude::*;

use crate::constants::{CALL_SEED, GAS_FEE_RECEIVER, PORTAL_SEED};
use crate::internal::send_call_internal;
use crate::state::{Call, CallType, Portal};

#[derive(Accounts)]
pub struct SendCall<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub authority: Signer<'info>,

    /// CHECK: This is the hardcoded gas fee receiver account.
    #[account(mut, address = GAS_FEE_RECEIVER)]
    pub gas_fee_receiver: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [PORTAL_SEED],
        bump,
    )]
    pub portal: Account<'info, Portal>,

    #[account(
        init,
        seeds = [CALL_SEED, portal.nonce.to_le_bytes().as_ref()],
        bump,
        payer = payer,
        space = 8 + Call::INIT_SPACE,
    )]
    pub call: Account<'info, Call>,

    pub system_program: Program<'info, System>,
}

pub fn send_call_handler(
    ctx: Context<SendCall>,
    ty: CallType,
    to: [u8; 20],
    gas_limit: u64,
    data: Vec<u8>,
) -> Result<()> {
    send_call_internal(
        &ctx.accounts.system_program,
        &ctx.accounts.payer,
        &ctx.accounts.authority,
        &ctx.accounts.gas_fee_receiver,
        &mut ctx.accounts.portal,
        &mut ctx.accounts.call,
        ty,
        to,
        gas_limit,
        0,
        data,
    )?;

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
            EIP1559_DEFAULT_WINDOW_DURATION_SECONDS, EIP1559_MINIMUM_BASE_FEE,
        },
        state::Eip1559,
        test_utils::{call_pda, mock_clock, mock_portal},
        ID as PORTAL_PROGRAM_ID,
    };

    #[test]
    fn test_send_call_fail_wrong_gas_fee_receiver() {
        let mut svm = LiteSVM::new();
        svm.add_program_from_file(PORTAL_PROGRAM_ID, "../../target/deploy/portal.so")
            .unwrap();

        // Create test accounts
        let payer = Keypair::new();
        let payer_pk = payer.pubkey();
        svm.airdrop(&payer_pk, LAMPORTS_PER_SOL).unwrap();

        let authority = Keypair::new();
        let authority_pk = authority.pubkey();

        // Use wrong gas fee receiver (not the expected GAS_FEE_RECEIVER)
        let wrong_gas_fee_receiver = Keypair::new().pubkey();

        // Test parameters
        let ty = CallType::Call;
        let to = [1u8; 20];
        let gas_limit = 100_000;
        let data = b"hello world".to_vec();

        // Mock the Portal account
        let portal_pda = mock_portal(
            &mut svm,
            Portal {
                nonce: 0,
                base_block_number: 0,
                eip1559: Eip1559::new(1000),
            },
        );

        let call_pda = call_pda(0);

        // Build the instruction with wrong gas fee receiver
        let send_calls_accounts = crate::accounts::SendCall {
            payer: payer_pk,
            authority: authority_pk,
            gas_fee_receiver: wrong_gas_fee_receiver, // This should fail
            portal: portal_pda,
            call: call_pda,
            system_program: solana_sdk_ids::system_program::ID,
        }
        .to_account_metas(None);

        let send_calls_ix = Instruction {
            program_id: PORTAL_PROGRAM_ID,
            accounts: send_calls_accounts,
            data: crate::instruction::SendCall {
                ty,
                to,
                gas_limit,
                data,
            }
            .data(),
        };

        // Build and send the transaction
        let tx = Transaction::new(
            &[&payer, &authority],
            Message::new(&[send_calls_ix], Some(&payer_pk)),
            svm.latest_blockhash(),
        );

        let result = svm.send_transaction(tx);
        assert!(
            result.is_err(),
            "Transaction should fail with wrong gas fee receiver"
        );
    }

    #[test]
    fn test_send_call_fail_creation_with_non_null_target() {
        let mut svm = LiteSVM::new();
        svm.add_program_from_file(PORTAL_PROGRAM_ID, "../../target/deploy/portal.so")
            .unwrap();

        // Create test accounts
        let payer = Keypair::new();
        let payer_pk = payer.pubkey();
        svm.airdrop(&payer_pk, LAMPORTS_PER_SOL).unwrap();

        let authority = Keypair::new();
        let authority_pk = authority.pubkey();

        // Test parameters - creation call with non-null target (should fail)
        let ty = CallType::Call;
        let to = [1u8; 20];
        let gas_limit = 100_000;
        let data = b"hello world".to_vec();

        // Mock the Portal account
        let portal_pda = mock_portal(
            &mut svm,
            Portal {
                nonce: 0,
                base_block_number: 0,
                eip1559: Eip1559::new(1000),
            },
        );

        let call_pda = call_pda(0);

        // Build the instruction
        let send_calls_accounts = crate::accounts::SendCall {
            payer: payer_pk,
            authority: authority_pk,
            gas_fee_receiver: GAS_FEE_RECEIVER,
            portal: portal_pda,
            call: call_pda,
            system_program: solana_sdk_ids::system_program::ID,
        }
        .to_account_metas(None);

        let send_calls_ix = Instruction {
            program_id: PORTAL_PROGRAM_ID,
            accounts: send_calls_accounts,
            data: crate::instruction::SendCall {
                ty,
                to,
                gas_limit,
                data,
            }
            .data(),
        };

        // Build and send the transaction
        let tx = Transaction::new(
            &[&payer, &authority],
            Message::new(&[send_calls_ix], Some(&payer_pk)),
            svm.latest_blockhash(),
        );

        let result = svm.send_transaction(tx);
        assert!(
            result.is_err(),
            "Transaction should fail when is_creation=true but target is not null address"
        );
    }

    #[test]
    fn test_send_call_fail_gas_limit_too_low() {
        let mut svm = LiteSVM::new();
        svm.add_program_from_file(PORTAL_PROGRAM_ID, "../../target/deploy/portal.so")
            .unwrap();

        // Create test accounts
        let payer = Keypair::new();
        let payer_pk = payer.pubkey();
        svm.airdrop(&payer_pk, LAMPORTS_PER_SOL).unwrap();

        let authority = Keypair::new();
        let authority_pk = authority.pubkey();

        // Test parameters - very low gas limit
        let ty = CallType::Call;
        let to = [1u8; 20];
        let gas_limit = 20_000;
        let data = b"this is a longer message that will require more gas".to_vec();

        // Mock the Portal account
        let portal_pda = mock_portal(
            &mut svm,
            Portal {
                nonce: 0,
                base_block_number: 0,
                eip1559: Eip1559::new(1000),
            },
        );

        let call_pda = call_pda(0);

        // Build the instruction
        let send_calls_accounts = crate::accounts::SendCall {
            payer: payer_pk,
            authority: authority_pk,
            gas_fee_receiver: GAS_FEE_RECEIVER,
            portal: portal_pda,
            call: call_pda,
            system_program: solana_sdk_ids::system_program::ID,
        }
        .to_account_metas(None);

        let send_calls_ix = Instruction {
            program_id: PORTAL_PROGRAM_ID,
            accounts: send_calls_accounts,
            data: crate::instruction::SendCall {
                ty,
                to,
                gas_limit,
                data,
            }
            .data(),
        };

        // Build and send the transaction
        let tx = Transaction::new(
            &[&payer, &authority],
            Message::new(&[send_calls_ix], Some(&payer_pk)),
            svm.latest_blockhash(),
        );

        let result = svm.send_transaction(tx);
        assert!(
            result.is_err(),
            "Transaction should fail when gas limit is too low"
        );
    }

    #[test]
    fn test_send_call_success() {
        let mut svm = LiteSVM::new();
        svm.add_program_from_file(PORTAL_PROGRAM_ID, "../../target/deploy/portal.so")
            .unwrap();

        // Create test accounts
        let payer = Keypair::new();
        let payer_pk = payer.pubkey();
        svm.airdrop(&payer_pk, LAMPORTS_PER_SOL).unwrap();

        let authority = Keypair::new();
        let authority_pk = authority.pubkey();

        // Test parameters
        let ty = CallType::Call;
        let to = [1u8; 20];
        let gas_limit = 200_000;
        let data = b"hello world".to_vec();

        // Mock the Portal account
        let initial_timestamp = 1000i64;
        let portal_pda = mock_portal(
            &mut svm,
            Portal {
                nonce: 0,
                base_block_number: 0,
                eip1559: Eip1559::new(initial_timestamp),
            },
        );

        // Mock clock with initial timestamp
        mock_clock(&mut svm, initial_timestamp);

        let call_pda = call_pda(0);

        // Build the instruction
        let send_calls_accounts = crate::accounts::SendCall {
            payer: payer_pk,
            authority: authority_pk,
            gas_fee_receiver: GAS_FEE_RECEIVER,
            portal: portal_pda,
            call: call_pda,
            system_program: solana_sdk_ids::system_program::ID,
        }
        .to_account_metas(None);

        let send_calls_ix = Instruction {
            program_id: PORTAL_PROGRAM_ID,
            accounts: send_calls_accounts,
            data: crate::instruction::SendCall {
                ty,
                to,
                gas_limit,
                data: data.clone(),
            }
            .data(),
        };

        // Build and send the transaction
        let tx = Transaction::new(
            &[&payer, &authority],
            Message::new(&[send_calls_ix], Some(&payer_pk)),
            svm.latest_blockhash(),
        );

        svm.send_transaction(tx)
            .expect("Transaction should succeed");

        // Verify that gas fee was transferred to the gas fee receiver
        let gas_fee_receiver_account = svm.get_account(&GAS_FEE_RECEIVER).unwrap();
        assert!(
            gas_fee_receiver_account.lamports > 0,
            "Gas fee receiver should have received lamports"
        );

        // Verify that the nonce was incremented
        let portal_account = svm.get_account(&portal_pda).unwrap();
        let portal_data = Portal::try_deserialize(&mut portal_account.data.as_slice()).unwrap();
        assert_eq!(portal_data.nonce, 1);

        // Verify that the call was created
        let call_account = svm.get_account(&call_pda).unwrap();
        let call_data = Call::try_deserialize(&mut call_account.data.as_slice()).unwrap();
        assert_eq!(call_data.nonce, 0);
        assert_eq!(call_data.ty, ty);
        assert_eq!(call_data.from, authority_pk);
        assert_eq!(call_data.to, to);
        assert_eq!(call_data.gas_limit, gas_limit);
        assert_eq!(call_data.remote_value, 0);
        assert_eq!(call_data.data, data);
    }

    #[test]
    fn test_dynamic_pricing_high_congestion_increases_base_fee() {
        let mut svm = LiteSVM::new();
        svm.add_program_from_file(PORTAL_PROGRAM_ID, "../../target/deploy/portal.so")
            .unwrap();

        // Mock clock with initial timestamp
        let initial_timestamp = 1000i64;
        mock_clock(&mut svm, initial_timestamp);

        // Mock Portal account with this timestamp
        let start_nonce = 0;
        let portal_pda = mock_portal(
            &mut svm,
            Portal {
                nonce: start_nonce,
                base_block_number: 0,
                eip1559: Eip1559::new(initial_timestamp),
            },
        );

        // Get initial state to understand the target
        let initial_base_fee = EIP1559_MINIMUM_BASE_FEE;
        let target_gas = EIP1559_DEFAULT_GAS_TARGET_PER_WINDOW;
        let window_duration = EIP1559_DEFAULT_WINDOW_DURATION_SECONDS;
        let denominator = EIP1559_DEFAULT_ADJUSTMENT_DENOMINATOR;

        // Create test accounts
        let payer = Keypair::new();
        let payer_pk = payer.pubkey();
        svm.airdrop(&payer_pk, 10 * LAMPORTS_PER_SOL).unwrap();

        let authority = Keypair::new();
        let authority_pk = authority.pubkey();

        // Do a bunch of transactions (high congestion - 2x target gas)
        let high_gas_per_tx = target_gas; // Each tx uses 100% of the target
        let num_transactions = 10; // Total: 10x target gas

        let gas_diff = num_transactions * high_gas_per_tx - target_gas;
        let expected_base_fee_increase = gas_diff / target_gas / denominator;

        for i in 0..num_transactions {
            let send_calls_accounts = crate::accounts::SendCall {
                payer: payer_pk,
                authority: authority_pk,
                gas_fee_receiver: GAS_FEE_RECEIVER,
                portal: portal_pda,
                call: call_pda(start_nonce + i),
                system_program: solana_sdk_ids::system_program::ID,
            };

            let send_calls_ix = Instruction {
                program_id: PORTAL_PROGRAM_ID,
                accounts: send_calls_accounts.to_account_metas(None),
                data: crate::instruction::SendCall {
                    ty: CallType::Call,
                    to: [1u8; 20],
                    gas_limit: high_gas_per_tx,
                    data: format!("high_congestion_tx_{}", i).into_bytes(),
                }
                .data(),
            };

            let tx = Transaction::new(
                &[&payer, &authority],
                Message::new(&[send_calls_ix], Some(&payer_pk)),
                svm.latest_blockhash(),
            );

            svm.send_transaction(tx)
                .expect("Transaction should succeed");
        }

        // Mock clock to pass the window afer transactions
        mock_clock(&mut svm, initial_timestamp + window_duration as i64);

        // Do one more transaction to trigger base fee update
        let trigger_accounts = crate::accounts::SendCall {
            payer: payer_pk,
            authority: authority_pk,
            gas_fee_receiver: GAS_FEE_RECEIVER,
            portal: portal_pda,
            call: call_pda(start_nonce + num_transactions),
            system_program: solana_sdk_ids::system_program::ID,
        };

        let trigger_ix = Instruction {
            program_id: PORTAL_PROGRAM_ID,
            accounts: trigger_accounts.to_account_metas(None),
            data: crate::instruction::SendCall {
                ty: CallType::Call,
                to: [1u8; 20],
                gas_limit: 200_000,
                data: b"trigger_update".to_vec(),
            }
            .data(),
        };

        let trigger_tx = Transaction::new(
            &[&payer, &authority],
            Message::new(&[trigger_ix], Some(&payer_pk)),
            svm.latest_blockhash(),
        );

        svm.send_transaction(trigger_tx)
            .expect("Trigger transaction should succeed");

        // Read the base fee from the Portal account
        let portal_account = svm.get_account(&portal_pda).unwrap();
        let final_base_fee = Portal::try_deserialize(&mut portal_account.data.as_slice())
            .unwrap()
            .eip1559
            .current_base_fee;

        // Verify that base fee increased as expected due to high congestion
        assert_eq!(
            final_base_fee,
            initial_base_fee + expected_base_fee_increase,
            "Base fee should increase as expected due to high congestion. Initial: {initial_base_fee}, Final: {final_base_fee}, Expected: {}",
            initial_base_fee + expected_base_fee_increase
        );
    }

    #[test]
    fn test_dynamic_pricing_no_usage_decreases_base_fee() {
        let mut svm = LiteSVM::new();
        svm.add_program_from_file(PORTAL_PROGRAM_ID, "../../target/deploy/portal.so")
            .unwrap();

        // Get constants for calculation
        let window_duration = EIP1559_DEFAULT_WINDOW_DURATION_SECONDS;

        // Mock clock with initial timestamp
        let initial_timestamp = 1000i64;
        mock_clock(&mut svm, initial_timestamp);

        // Mock Portal account with high base fee
        let high_base_fee = 100u64; // 100 GWEI in wei
        let mut eip1559 = Eip1559::new(initial_timestamp);
        eip1559.current_base_fee = high_base_fee;
        let portal_pda = mock_portal(
            &mut svm,
            Portal {
                nonce: 0,
                base_block_number: 0,
                eip1559,
            },
        );

        let call_pda = call_pda(0);

        // Mock clock to be 10 time windows later (10 seconds later)
        let windows_passed = window_duration * 10;
        let new_timestamp = initial_timestamp + windows_passed as i64;
        mock_clock(&mut svm, new_timestamp);

        // Create test accounts
        let payer = Keypair::new();
        let payer_pk = payer.pubkey();
        svm.airdrop(&payer_pk, LAMPORTS_PER_SOL).unwrap();

        let authority = Keypair::new();
        let authority_pk = authority.pubkey();

        // Do a single transaction to trigger base fee update
        let accounts = crate::accounts::SendCall {
            payer: payer_pk,
            authority: authority_pk,
            gas_fee_receiver: GAS_FEE_RECEIVER,
            portal: portal_pda,
            call: call_pda,
            system_program: solana_sdk_ids::system_program::ID,
        };

        let ix = Instruction {
            program_id: PORTAL_PROGRAM_ID,
            accounts: accounts.to_account_metas(None),
            data: crate::instruction::SendCall {
                ty: CallType::Call,
                to: [1u8; 20],
                gas_limit: 200_000,
                data: b"trigger_update".to_vec(),
            }
            .data(),
        };

        let tx = Transaction::new(
            &[&payer, &authority],
            Message::new(&[ix], Some(&payer_pk)),
            svm.latest_blockhash(),
        );

        svm.send_transaction(tx)
            .expect("Transaction should succeed");

        // Read the new base fee
        let portal_account = svm.get_account(&portal_pda).unwrap();
        let portal_data = Portal::try_deserialize(&mut portal_account.data.as_slice()).unwrap();

        // Verify it actually decreased
        assert!(
            portal_data.eip1559.current_base_fee < high_base_fee,
            "Base fee should have decreased from initial high value"
        );
    }
}
