use anchor_lang::{prelude::*, solana_program::keccak};

use crate::{
    constants::REMOTE_CALL_SEED,
    internal::{merkle_utils, Proof},
    state::{OutputRoot, RemoteCall},
};

#[derive(Accounts)]
#[instruction(call_hash: [u8; 32])]
pub struct ProveCall<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + RemoteCall::INIT_SPACE,
        // NOTE: We check that the PDA derivation is correct in the handler to optimize the CPI.
        // seeds = [REMOTE_CALL_SEED, &call_hash],
        // bump
    )]
    pub remote_call: Account<'info, RemoteCall>,

    pub output_root: Account<'info, OutputRoot>,

    pub system_program: Program<'info, System>,
}

pub fn prove_call_handler(
    ctx: Context<ProveCall>,
    nonce: [u8; 32],
    sender: [u8; 20],
    data: Vec<u8>,
    proof: Proof,
) -> Result<()> {
    // Hash the call
    let call_hash = hash_call(&nonce, &sender, &data);

    // Verify the PDA derivation is correct
    let (remote_call_pda, _) =
        Pubkey::find_program_address(&[REMOTE_CALL_SEED, &call_hash], ctx.program_id);
    require!(
        remote_call_pda == ctx.accounts.remote_call.key(),
        ProveCallError::InvalidPda
    );

    // Verify the merkle proof to ensure the transaction exists on the source chain
    merkle_utils::verify_mmr_proof(&ctx.accounts.output_root.root, &call_hash, &proof)?;

    ctx.accounts.remote_call.executed = false;
    ctx.accounts.remote_call.sender = sender;
    ctx.accounts.remote_call.data = data;

    Ok(())
}

fn hash_call(nonce: &[u8; 32], sender: &[u8; 20], data: &[u8]) -> [u8; 32] {
    let mut data_to_hash = Vec::new();
    data_to_hash.extend_from_slice(nonce);
    data_to_hash.extend_from_slice(sender);
    data_to_hash.extend_from_slice(data);

    keccak::hash(&data_to_hash).0
}

#[error_code]
pub enum ProveCallError {
    #[msg("Invalid PDA derivation")]
    InvalidPda,
    #[msg("Invalid proof")]
    InvalidProof,
}
