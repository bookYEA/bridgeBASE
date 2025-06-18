use anchor_lang::prelude::*;

use crate::constants::{EIP1559_SEED, GAS_COST_SCALER, GAS_COST_SCALER_DP, GAS_FEE_RECEIVER};
use crate::instructions::CallType;
use crate::state::Eip1559;

use super::Call;

#[derive(Accounts)]
pub struct SendCall<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub authority: Signer<'info>,

    /// CHECK: This is the hardcoded gas fee receiver account.
    #[account(mut, address = GAS_FEE_RECEIVER @ SendCallError::IncorrectGasFeeReceiver)]
    pub gas_fee_receiver: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [EIP1559_SEED],
        bump,
    )]
    pub eip1559: Account<'info, Eip1559>,

    pub system_program: Program<'info, System>,
}

#[event]
pub struct CallSent {
    pub from: Pubkey,
    pub to: [u8; 20],
    pub opaque_data: Vec<u8>,
}

pub fn send_call_handler(
    ctx: Context<SendCall>,
    ty: CallType,
    to: [u8; 20],
    min_gas_limit: u64,
    data: Vec<u8>,
) -> Result<()> {
    send_call(
        &ctx.accounts.system_program,
        &ctx.accounts.payer,
        &ctx.accounts.gas_fee_receiver,
        &mut ctx.accounts.eip1559,
        Call {
            ty,
            from: ctx.accounts.authority.key(),
            to,
            min_gas_limit,
            data,
        },
    )
}

pub fn send_call<'info>(
    system_program: &Program<'info, System>,
    payer: &Signer<'info>,
    gas_fee_receiver: &AccountInfo<'info>,
    eip1559: &mut Account<'info, Eip1559>,
    call: Call,
) -> Result<()> {
    // TODO: The `relayCall` function on Base expects a `nonce`. Figure out where and how to generate it.

    let Call {
        ty,
        from,
        to,
        min_gas_limit,
        data,
    } = call;

    // Ensure no target address is provided for contract creation
    require!(
        matches!(ty, CallType::Call | CallType::DelegateCall) || to == [0; 20],
        SendCallError::CreationWithNonZeroTarget
    );

    // Calculate the effective minimum gas limit to provide to ensure a successful relay
    // of a call with `min_gas_limit` gas.
    let effective_min_gas_limit = base_gas(&data, min_gas_limit);

    let opaque_data = {
        let mut opaque_data = vec![];
        opaque_data.push(ty as u8);
        opaque_data.extend_from_slice(&effective_min_gas_limit.to_le_bytes());
        opaque_data.extend_from_slice(&data);
        opaque_data
    };

    // Pay for the gas to relay the call on Base.
    pay_for_gas(
        system_program,
        payer,
        gas_fee_receiver,
        eip1559,
        effective_min_gas_limit,
    )?;

    emit!(CallSent {
        from,
        to,
        opaque_data,
    });

    Ok(())
}

fn base_gas(data: &[u8], min_gas_limit: u64) -> u64 {
    const RELAY_CONSTANT_OVERHEAD_GAS: u64 = 200_000; // Constant overhead added to the base gas to relay a call.
    const RELAY_CALL_OVERHEAD_GAS: u64 = 40_000; // Covers dynamic parts of the CALL opcode
    const RELAY_CALL_CHECK_BUFFER_GAS: u64 = 5_000; // Buffer between _hasMinGas check and the CALL
    const RELAY_CALL_POST_EXECUTION_RESERVED_GAS: u64 = 40_000; // Ensures execution of relayCall completes after call.

    const TX_BASE_GAS: u64 = 21_000;
    const MIN_GAS_CALLDATA_OVERHEAD: u64 = 16; // Extra gas added to base gas for each byte of calldata in a message.
    const FLOOR_CALLDATA_OVERHEAD: u64 = 40; // Floor overhead per byte of non-zero calldata in a message. Calldata floor was introduced in EIP-7623.

    let execution_gas = RELAY_CONSTANT_OVERHEAD_GAS
        + RELAY_CALL_OVERHEAD_GAS
        + RELAY_CALL_CHECK_BUFFER_GAS
        + RELAY_CALL_POST_EXECUTION_RESERVED_GAS
        + min_gas_limit * 63 / 64;

    // TODO: The tx size on Base will be bigger as it's wrapped in a call to the `relayCall` function.
    let tx_size = data.len() as u64;

    // TODO: More thought is needed here as it is possible to do contract creation
    //       Taken from: https://github.com/ethereum-optimism/optimism/blob/8261ca8e540558224912d61be8f502cf3e1e3dc5/packages/contracts-bedrock/src/universal/CrossDomainMessenger.sol#L389
    TX_BASE_GAS
        + (execution_gas + tx_size * MIN_GAS_CALLDATA_OVERHEAD)
            .max(tx_size * FLOOR_CALLDATA_OVERHEAD)
}

fn pay_for_gas<'info>(
    system_program: &Program<'info, System>,
    payer: &Signer<'info>,
    gas_fee_receiver: &AccountInfo<'info>,
    eip1559: &mut Account<'info, Eip1559>,
    effective_min_gas_limit: u64,
) -> Result<()> {
    // Get the base fee for the current window
    let current_timestamp = Clock::get()?.unix_timestamp;
    let base_fee = eip1559.refresh_base_fee(current_timestamp);

    // Record gas usage for this transaction
    eip1559.add_gas_usage(effective_min_gas_limit);

    let gas_cost = effective_min_gas_limit * base_fee * GAS_COST_SCALER / GAS_COST_SCALER_DP;

    let cpi_ctx = CpiContext::new(
        system_program.to_account_info(),
        anchor_lang::system_program::Transfer {
            from: payer.to_account_info(),
            to: gas_fee_receiver.clone(),
        },
    );
    anchor_lang::system_program::transfer(cpi_ctx, gas_cost)?;

    Ok(())
}

#[error_code]
pub enum SendCallError {
    #[msg("Incorrect gas fee receiver")]
    IncorrectGasFeeReceiver,
    #[msg("Creation with non-zero target")]
    CreationWithNonZeroTarget,
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
        test_utils::{mock_clock, mock_eip1559},
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
        let gas_limit = 100_000u64;
        let data = b"hello world".to_vec();

        // Mock the EIP1559 account
        let eip1559_pda = mock_eip1559(&mut svm, Eip1559::new(1000));

        // Build the instruction with wrong gas fee receiver
        let send_call_accounts = crate::accounts::SendCall {
            payer: payer_pk,
            authority: authority_pk,
            gas_fee_receiver: wrong_gas_fee_receiver, // This should fail
            eip1559: eip1559_pda,
            system_program: solana_sdk_ids::system_program::ID,
        }
        .to_account_metas(None);

        let send_call_ix = Instruction {
            program_id: PORTAL_PROGRAM_ID,
            accounts: send_call_accounts,
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
            Message::new(&[send_call_ix], Some(&payer_pk)),
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
        let ty = CallType::Create;
        let to = [1u8; 20]; // Non-null address
        let gas_limit = 100_000u64;
        let data = b"hello world".to_vec();

        // Mock the EIP1559 account
        let eip1559_pda = mock_eip1559(&mut svm, Eip1559::new(1000));

        // Build the instruction
        let send_call_accounts = crate::accounts::SendCall {
            payer: payer_pk,
            authority: authority_pk,
            gas_fee_receiver: GAS_FEE_RECEIVER,
            eip1559: eip1559_pda,
            system_program: solana_sdk_ids::system_program::ID,
        }
        .to_account_metas(None);

        let send_call_ix = Instruction {
            program_id: PORTAL_PROGRAM_ID,
            accounts: send_call_accounts,
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
            Message::new(&[send_call_ix], Some(&payer_pk)),
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
        let gas_limit = 1u64; // Extremely low gas limit that should fail
        let data = b"this is a longer message that will require more gas".to_vec();

        // Mock the EIP1559 account
        let eip1559_pda = mock_eip1559(&mut svm, Eip1559::new(1000));

        // Build the instruction
        let send_call_accounts = crate::accounts::SendCall {
            payer: payer_pk,
            authority: authority_pk,
            gas_fee_receiver: GAS_FEE_RECEIVER,
            eip1559: eip1559_pda,
            system_program: solana_sdk_ids::system_program::ID,
        }
        .to_account_metas(None);

        let send_call_ix = Instruction {
            program_id: PORTAL_PROGRAM_ID,
            accounts: send_call_accounts,
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
            Message::new(&[send_call_ix], Some(&payer_pk)),
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
        let to = [1u8; 20]; // Sample target address
        let gas_limit = 100_000u64;
        let data = b"hello world".to_vec();

        // Mock the EIP1559 account
        let initial_timestamp = 1000i64;
        let eip1559_pda = mock_eip1559(&mut svm, Eip1559::new(initial_timestamp));

        // Mock clock with initial timestamp
        mock_clock(&mut svm, initial_timestamp);

        // Build the instruction
        let send_call_accounts = crate::accounts::SendCall {
            payer: payer_pk,
            authority: authority_pk,
            gas_fee_receiver: GAS_FEE_RECEIVER,
            eip1559: eip1559_pda,
            system_program: solana_sdk_ids::system_program::ID,
        }
        .to_account_metas(None);

        let send_call_ix = Instruction {
            program_id: PORTAL_PROGRAM_ID,
            accounts: send_call_accounts,
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
            Message::new(&[send_call_ix], Some(&payer_pk)),
            svm.latest_blockhash(),
        );

        // TODO: Check that the correct event is emitted
        svm.send_transaction(tx)
            .expect("Transaction should succeed");

        // Verify that gas fee was transferred to the gas fee receiver
        let gas_fee_receiver_account = svm.get_account(&GAS_FEE_RECEIVER).unwrap();
        assert!(
            gas_fee_receiver_account.lamports > 0,
            "Gas fee receiver should have received lamports"
        );
    }

    #[test]
    fn test_dynamic_pricing_high_congestion_increases_base_fee() {
        let mut svm = LiteSVM::new();
        svm.add_program_from_file(PORTAL_PROGRAM_ID, "../../target/deploy/portal.so")
            .unwrap();

        // Mock clock with initial timestamp
        let initial_timestamp = 1000i64;
        mock_clock(&mut svm, initial_timestamp);

        // Mock EIP1559 account with this timestamp
        let eip1559_pda = mock_eip1559(&mut svm, Eip1559::new(initial_timestamp));

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
            let send_call_accounts = crate::accounts::SendCall {
                payer: payer_pk,
                authority: authority_pk,
                gas_fee_receiver: GAS_FEE_RECEIVER,
                eip1559: eip1559_pda,
                system_program: solana_sdk_ids::system_program::ID,
            };

            let send_call_ix = Instruction {
                program_id: PORTAL_PROGRAM_ID,
                accounts: send_call_accounts.to_account_metas(None),
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
                Message::new(&[send_call_ix], Some(&payer_pk)),
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
            eip1559: eip1559_pda,
            system_program: solana_sdk_ids::system_program::ID,
        };

        let trigger_ix = Instruction {
            program_id: PORTAL_PROGRAM_ID,
            accounts: trigger_accounts.to_account_metas(None),
            data: crate::instruction::SendCall {
                ty: CallType::Call,
                to: [1u8; 20],
                gas_limit: 100_000,
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

        // Read the base fee from the EIP1559 account
        let final_account = svm.get_account(&eip1559_pda).unwrap();
        let final_eip1559 = Eip1559::try_deserialize(&mut final_account.data.as_slice()).unwrap();
        let final_base_fee = final_eip1559.current_base_fee;

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

        // Mock EIP1559 account with high base fee
        let high_base_fee = 100u64; // 100 GWEI in wei
        let mut eip1559 = Eip1559::new(initial_timestamp);
        eip1559.current_base_fee = high_base_fee;
        let eip1559_pda = mock_eip1559(&mut svm, eip1559);

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
            eip1559: eip1559_pda,
            system_program: solana_sdk_ids::system_program::ID,
        };

        let ix = Instruction {
            program_id: PORTAL_PROGRAM_ID,
            accounts: accounts.to_account_metas(None),
            data: crate::instruction::SendCall {
                ty: CallType::Call,
                to: [1u8; 20],
                gas_limit: 100_000,
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

        // Read the base fee from the EIP1559 account
        let final_account = svm.get_account(&eip1559_pda).unwrap();
        let final_eip1559 = Eip1559::try_deserialize(&mut final_account.data.as_slice()).unwrap();
        let final_base_fee = final_eip1559.current_base_fee;

        // Verify it actually decreased
        assert!(
            final_base_fee < high_base_fee,
            "Base fee should have decreased from initial high value"
        );
    }
}
