use anchor_lang::prelude::*;

use crate::{Messenger, OutputRoot, MESSENGER_SEED, OUTPUT_ROOT_SEED, TRUSTED_ORACLE};

/// Account structure for posting an output root to the bridge
/// 
/// This instruction creates a new OutputRoot account and updates the messenger
/// with the latest processed block number. The root serves as a commitment
/// to the L2 state at a specific block height.
#[derive(Accounts)]
#[instruction(_output_root: [u8; 32], block_number: u64)]
pub struct PostRoot<'info> {
    /// The output root account being created
    /// 
    /// This account stores the merkle root and associated block number.
    /// It's derived using a PDA with seeds [OUTPUT_ROOT_SEED, block_number]
    /// to ensure uniqueness per block and prevent replay attacks.
    #[account(
        init, 
        payer = payer, 
        space = 8 + OutputRoot::INIT_SPACE, 
        seeds = [OUTPUT_ROOT_SEED, &block_number.to_le_bytes()], 
        bump
    )]
    pub root: Account<'info, OutputRoot>,

    /// The messenger account that tracks the latest processed block
    /// 
    /// This account maintains global state about the bridge including
    /// the highest block number that has been processed. It's used
    /// to enforce ordering and prevent processing old blocks.
    #[account(mut, seeds = [MESSENGER_SEED], bump = messenger.bump)]
    pub messenger: Account<'info, Messenger>,

    /// The trusted oracle account authorized to post roots
    /// 
    /// Only accounts matching TRUSTED_ORACLE address can execute this instruction.
    /// This prevents unauthorized parties from posting malicious or incorrect
    /// state commitments to the bridge.
    #[account(mut, address = TRUSTED_ORACLE)]
    pub payer: Signer<'info>,

    /// System program required for account creation
    pub system_program: Program<'info, System>,
}

/// Submits an output root for a specific Base block
/// 
/// This function creates a new OutputRoot account containing the MMR root
/// and block number, then updates the messenger's latest block tracking.
/// 
/// # Arguments
/// 
/// * `ctx` - The instruction context containing all required accounts
/// * `output_root` - The 32-byte MMR root representing the Base state commitment
/// * `block_number` - The Base block number this root corresponds to
pub fn submit_root_handler(
    ctx: Context<PostRoot>,
    output_root: [u8; 32],
    block_number: u64,
) -> Result<()> {
    ctx.accounts.root.root = output_root;
    ctx.accounts.root.block_number = block_number;
    ctx.accounts.messenger.latest_block_number = block_number;
    Ok(())
}
