use anchor_lang::{
    prelude::*,
    system_program::{self, Transfer},
};

use portal::constants::PORTAL_AUTHORITY_SEED;

use crate::constants::{REMOTE_BRIDGE, SOL_VAULT_SEED};

#[derive(Accounts)]
#[instruction(remote_token: [u8; 20])]
pub struct FinalizeBridgeSol<'info> {
    /// CHECK: This is the Portal authority account.
    ///        It ensures that the call is triggered by the Portal program from an expected
    ///        remote sender (REMOTE_BRIDGE here).
    #[account(
        seeds = [PORTAL_AUTHORITY_SEED, REMOTE_BRIDGE.as_ref()],
        bump,
        seeds::program = portal::program::Portal::id()
    )]
    pub portal_authority: Signer<'info>,

    /// CHECK: This is the sol vault account for a specific remote token.
    #[account(mut, seeds = [SOL_VAULT_SEED, remote_token.as_ref()], bump)]
    pub sol_vault: AccountInfo<'info>,

    /// CHECK: This is the account to send the SOL to.
    #[account(mut)]
    pub to: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

pub fn finalize_bridge_sol_handler(
    ctx: Context<FinalizeBridgeSol>,
    remote_token: [u8; 20],
    amount: u64,
) -> Result<()> {
    let seeds: &[&[&[u8]]] = &[&[
        SOL_VAULT_SEED,
        remote_token.as_ref(),
        &[ctx.bumps.sol_vault],
    ]];

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.system_program.to_account_info(),
        Transfer {
            from: ctx.accounts.sol_vault.to_account_info(),
            to: ctx.accounts.to.to_account_info(),
        },
        seeds,
    );
    system_program::transfer(cpi_ctx, amount)
}

#[cfg(test)]
mod tests {
    use super::*;

    use anchor_lang::{solana_program::native_token::LAMPORTS_PER_SOL, InstructionData};
    use litesvm::LiteSVM;
    use portal::internal::Ix;
    use solana_instruction::Instruction;
    use solana_keypair::Keypair;
    use solana_message::Message;
    use solana_signer::Signer;
    use solana_transaction::Transaction;

    use crate::{
        test_utils::{mock_remote_call, mock_sol_vault, portal_authority},
        ID as TOKEN_BRIDGE_PROGRAM_ID,
    };
    use portal::ID as PORTAL_PROGRAM_ID;

    #[test]
    fn test_finalize_bridge_sol_success() {
        let mut svm = LiteSVM::new();
        svm.add_program_from_file(
            TOKEN_BRIDGE_PROGRAM_ID,
            "../../target/deploy/token_bridge.so",
        )
        .unwrap();

        svm.add_program_from_file(PORTAL_PROGRAM_ID, "../../target/deploy/portal.so")
            .unwrap();

        // Test parameters
        let remote_token = [0x42u8; 20]; // Sample remote token address
        let bridge_amount = 5 * LAMPORTS_PER_SOL; // 5 SOL to bridge back
        let vault_initial_balance = 10 * LAMPORTS_PER_SOL; // Vault starts with 10 SOL

        // Create recipient account
        let payer = Keypair::new();
        svm.airdrop(&payer.pubkey(), LAMPORTS_PER_SOL).unwrap();

        let recipient = Keypair::new();
        let recipient_pk = recipient.pubkey();

        // Create and fund the SOL vault
        let sol_vault = mock_sol_vault(&mut svm, remote_token, vault_initial_balance);

        // Create portal authority
        let portal_authority = portal_authority();

        // Build the TokenBridge's finalize_bridge_sol instruction
        let finalize_bridge_sol_accounts = crate::accounts::FinalizeBridgeSol {
            portal_authority,
            sol_vault,
            to: recipient_pk,
            system_program: solana_sdk_ids::system_program::ID,
        }
        .to_account_metas(None)
        .into_iter()
        .skip(1) // Skip portal_authority since relay_call handles it
        .collect::<Vec<_>>();

        let finalize_bridge_sol_ix = Ix::from(Instruction {
            program_id: TOKEN_BRIDGE_PROGRAM_ID,
            accounts: finalize_bridge_sol_accounts.clone(),
            data: crate::instruction::FinalizeBridgeSol {
                remote_token,
                amount: bridge_amount,
            }
            .data(),
        });

        // Build the Portal's relay_call instruction
        let remote_call = mock_remote_call(
            &mut svm,
            REMOTE_BRIDGE,
            vec![finalize_bridge_sol_ix].try_to_vec().unwrap(),
            false,
        );

        let mut relay_call_accounts = portal::accounts::RelayCall {
            portal_authority,
            payer: payer.pubkey(),
            remote_call,
        }
        .to_account_metas(None);

        // Don't forget to add the finalize_bridge_sol accounts (and the) to the relay_call instruction.
        relay_call_accounts.extend_from_slice(&finalize_bridge_sol_accounts);
        relay_call_accounts.push(AccountMeta::new_readonly(TOKEN_BRIDGE_PROGRAM_ID, false));

        let relay_call_ix = Instruction {
            program_id: PORTAL_PROGRAM_ID,
            accounts: relay_call_accounts,
            data: portal::instruction::RelayCall {}.data(),
        };

        // Build and send the transaction
        let tx = Transaction::new(
            &[&payer],
            Message::new(&[relay_call_ix], Some(&payer.pubkey())),
            svm.latest_blockhash(),
        );

        svm.send_transaction(tx)
            .expect("Transaction should succeed");

        let recipient_account = svm.get_account(&recipient_pk).unwrap();
        assert_eq!(recipient_account.lamports, bridge_amount);

        let sol_vault_account = svm.get_account(&sol_vault).unwrap();
        assert_eq!(
            sol_vault_account.lamports,
            vault_initial_balance - bridge_amount
        );
    }
}
