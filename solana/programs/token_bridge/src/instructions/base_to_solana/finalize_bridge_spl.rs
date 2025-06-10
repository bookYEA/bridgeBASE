use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, TokenAccount, TokenInterface, TransferChecked};

use portal::constants::PORTAL_AUTHORITY_SEED;

use crate::constants::{REMOTE_BRIDGE, TOKEN_VAULT_SEED};

#[derive(Accounts)]
#[instruction(remote_token: [u8; 20])]
pub struct FinalizeBridgeSpl<'info> {
    /// CHECK: This is the Portal authority account.
    ///        It ensures that the call is triggered by the Portal program from an expected
    ///        remote sender (REMOTE_BRIDGE here).
    #[account(
        seeds = [PORTAL_AUTHORITY_SEED, REMOTE_BRIDGE.as_ref()],
        bump,
        seeds::program = portal::program::Portal::id()
    )]
    pub portal_authority: Signer<'info>,

    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        seeds = [TOKEN_VAULT_SEED, mint.key().as_ref(), remote_token.as_ref()],
        bump,
    )]
    pub token_vault: InterfaceAccount<'info, TokenAccount>,

    #[account(mut)]
    pub to_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn finalize_bridge_spl_handler(
    ctx: Context<FinalizeBridgeSpl>,
    remote_token: [u8; 20],
    amount: u64,
) -> Result<()> {
    let mint_key = ctx.accounts.mint.key();
    let seeds: &[&[&[u8]]] = &[&[
        TOKEN_VAULT_SEED,
        mint_key.as_ref(),
        remote_token.as_ref(),
        &[ctx.bumps.token_vault],
    ]];

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        TransferChecked {
            mint: ctx.accounts.mint.to_account_info(),
            from: ctx.accounts.token_vault.to_account_info(),
            to: ctx.accounts.to_token_account.to_account_info(),
            authority: ctx.accounts.token_vault.to_account_info(),
        },
        seeds,
    );
    token_interface::transfer_checked(cpi_ctx, amount, ctx.accounts.mint.decimals)
}

#[cfg(test)]
mod tests {
    use anchor_lang::{prelude::*, InstructionData};
    use anchor_spl::token::spl_token::state::Account as TokenAccount;
    use litesvm::LiteSVM;
    use solana_instruction::Instruction;
    use solana_keypair::Keypair;
    use solana_message::Message;
    use solana_program_pack::Pack;
    use solana_signer::Signer;

    use portal::{internal::Ix, ID as PORTAL_PROGRAM_ID};
    use solana_transaction::Transaction;

    use crate::{
        constants::REMOTE_BRIDGE,
        test_utils::{
            mock_mint, mock_remote_call, mock_token_account, mock_token_vault, portal_authority,
            SPL_TOKEN_PROGRAM_ID,
        },
        ID as TOKEN_BRIDGE_PROGRAM_ID,
    };

    const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

    #[test]
    fn test_finalize_bridge_spl_success() {
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
        let decimals = 6u8; // USDC-like decimals
        let bridge_amount = 1000 * 10_u64.pow(decimals as u32); // 1000 tokens
        let vault_initial_balance = 10000 * 10_u64.pow(decimals as u32); // 10000 tokens

        // Create payer
        let payer = Keypair::new();
        svm.airdrop(&payer.pubkey(), LAMPORTS_PER_SOL).unwrap();

        // Create recipient
        let recipient = Keypair::new();
        let recipient_pk = recipient.pubkey();

        // Create mint
        let mint = Keypair::new().pubkey();
        mock_mint(&mut svm, mint, decimals);

        // Create token vault with funds
        let token_vault = mock_token_vault(&mut svm, mint, remote_token, vault_initial_balance);

        // Create destination token account
        let to_token_account = Keypair::new().pubkey();
        mock_token_account(&mut svm, to_token_account, mint, recipient_pk, 0);

        // Compute the portal authority PDA
        let portal_authority = portal_authority();

        // Build the TokenBridge's finalize_bridge_spl instruction
        let finalize_bridge_spl_accounts = crate::accounts::FinalizeBridgeSpl {
            portal_authority,
            mint,
            token_vault,
            to_token_account,
            token_program: SPL_TOKEN_PROGRAM_ID,
        }
        .to_account_metas(None)
        .into_iter()
        .skip(1) // Skip portal_authority since relay_call handles it
        .collect::<Vec<_>>();

        let finalize_bridge_spl_ix = Ix::from(Instruction {
            program_id: TOKEN_BRIDGE_PROGRAM_ID,
            accounts: finalize_bridge_spl_accounts.clone(),
            data: crate::instruction::FinalizeBridgeSpl {
                remote_token,
                amount: bridge_amount,
            }
            .data(),
        });

        // Build the Portal's relay_call instruction
        let remote_call = mock_remote_call(
            &mut svm,
            REMOTE_BRIDGE,
            vec![finalize_bridge_spl_ix].try_to_vec().unwrap(),
            false,
        );

        let mut relay_call_accounts = portal::accounts::RelayCall {
            portal_authority,
            payer: payer.pubkey(),
            remote_call,
        }
        .to_account_metas(None);

        // Add the finalize_bridge_spl accounts and token program to the relay_call instruction
        relay_call_accounts.extend_from_slice(&finalize_bridge_spl_accounts);
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

        // Verify that tokens were transferred from vault to recipient
        let to_token_account_after = svm.get_account(&to_token_account).unwrap();
        let to_token_account_after = TokenAccount::unpack(&to_token_account_after.data).unwrap();
        assert_eq!(
            to_token_account_after.amount, bridge_amount,
            "Recipient should receive the bridged tokens"
        );

        let token_vault_after = svm.get_account(&token_vault).unwrap();
        let token_vault_after = TokenAccount::unpack(&token_vault_after.data).unwrap();
        assert_eq!(
            token_vault_after.amount,
            vault_initial_balance - bridge_amount,
            "Vault should have reduced balance"
        );
    }
}
