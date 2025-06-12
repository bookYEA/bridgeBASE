use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, MintToChecked, Token2022, TokenAccount};

use portal::constants::PORTAL_AUTHORITY_SEED;

use crate::{
    constants::{REMOTE_BRIDGE, WRAPPED_TOKEN_SEED},
    instructions::PartialTokenMetadata,
};

#[derive(Accounts)]
pub struct FinalizeBridgeToken<'info> {
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

    #[account(mut)]
    pub to_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Program<'info, Token2022>,
}

pub fn finalize_bridge_token_handler(ctx: Context<FinalizeBridgeToken>, amount: u64) -> Result<()> {
    let partial_token_metadata =
        PartialTokenMetadata::try_from(&ctx.accounts.mint.to_account_info())?;

    let decimals_bytes = ctx.accounts.mint.decimals.to_le_bytes();
    let metadata_hash = partial_token_metadata.hash();

    let seeds: &[&[u8]] = &[
        WRAPPED_TOKEN_SEED,
        decimals_bytes.as_ref(),
        metadata_hash.as_ref(),
    ];
    let (_, mint_bump) = Pubkey::find_program_address(seeds, ctx.program_id);
    let seeds: &[&[&[u8]]] = &[&[
        WRAPPED_TOKEN_SEED,
        decimals_bytes.as_ref(),
        metadata_hash.as_ref(),
        &[mint_bump],
    ]];

    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        MintToChecked {
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.to_token_account.to_account_info(),
            authority: ctx.accounts.mint.to_account_info(),
        },
        seeds,
    );
    token_interface::mint_to_checked(cpi_ctx, amount, ctx.accounts.mint.decimals)
}

#[cfg(test)]
mod tests {
    use anchor_lang::{
        prelude::*, solana_program::native_token::LAMPORTS_PER_SOL, InstructionData,
    };
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
        instructions::PartialTokenMetadata,
        test_utils::{
            mock_remote_call, mock_token_account, mock_wrapped_mint, portal_authority,
            SPL_TOKEN_PROGRAM_ID,
        },
        ID as TOKEN_BRIDGE_PROGRAM_ID,
    };

    #[test]
    fn test_finalize_bridge_token_success() {
        let mut svm = LiteSVM::new();
        svm.add_program_from_file(
            TOKEN_BRIDGE_PROGRAM_ID,
            "../../target/deploy/token_bridge.so",
        )
        .unwrap();
        svm.add_program_from_file(PORTAL_PROGRAM_ID, "../../target/deploy/portal.so")
            .unwrap();

        // Test parameters
        let partial_token_metadata = PartialTokenMetadata {
            remote_token: [0x42u8; 20],
            name: "Sample Token".to_string(),
            symbol: "STK".to_string(),
        };
        let decimals = 6u8; // USDC-like decimals
        let mint_amount = 1000 * 10_u64.pow(decimals as u32); // 1000 tokens to mint

        // Create payer
        let payer = Keypair::new();
        svm.airdrop(&payer.pubkey(), LAMPORTS_PER_SOL).unwrap();

        // Create recipient
        let recipient = Keypair::new();
        let recipient_pk = recipient.pubkey();

        // Create wrapped mint for the remote token
        let wrapped_mint = mock_wrapped_mint(&mut svm, decimals, &partial_token_metadata);

        // Create destination token account (starts with 0 tokens)
        let to_token_account = Keypair::new().pubkey();
        mock_token_account(&mut svm, to_token_account, wrapped_mint, recipient_pk, 0);

        // Compute the portal authority PDA
        let portal_authority = portal_authority();

        // Build the TokenBridge's finalize_bridge_token instruction
        let finalize_bridge_token_accounts = crate::accounts::FinalizeBridgeToken {
            portal_authority,
            mint: wrapped_mint,
            to_token_account,
            token_program: SPL_TOKEN_PROGRAM_ID,
        }
        .to_account_metas(None)
        .into_iter()
        .skip(1) // Skip portal_authority since relay_call handles it
        .collect::<Vec<_>>();

        let finalize_bridge_token_ix = Ix::from(Instruction {
            program_id: TOKEN_BRIDGE_PROGRAM_ID,
            accounts: finalize_bridge_token_accounts.clone(),
            data: crate::instruction::FinalizeBridgeToken {
                amount: mint_amount,
            }
            .data(),
        });

        // Build the Portal's relay_call instruction
        let remote_call = mock_remote_call(
            &mut svm,
            REMOTE_BRIDGE,
            vec![finalize_bridge_token_ix].try_to_vec().unwrap(),
            false,
        );

        let mut relay_call_accounts = portal::accounts::RelayCall {
            portal_authority,
            payer: payer.pubkey(),
            remote_call,
        }
        .to_account_metas(None);

        // Add the finalize_bridge_token accounts and token program to the relay_call instruction
        relay_call_accounts.extend_from_slice(&finalize_bridge_token_accounts);
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

        // Verify that tokens were minted to the recipient
        let to_token_account_after = svm.get_account(&to_token_account).unwrap();
        let to_token_account_after = TokenAccount::unpack(&to_token_account_after.data).unwrap();
        assert_eq!(
            to_token_account_after.amount, mint_amount,
            "Recipient should receive the minted wrapped tokens"
        );
    }
}
