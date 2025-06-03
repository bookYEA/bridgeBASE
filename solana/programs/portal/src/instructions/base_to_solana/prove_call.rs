use anchor_lang::{prelude::*, solana_program::keccak};

use crate::{
    constants::REMOTE_CALL_SEED,
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
        seeds = [REMOTE_CALL_SEED, &call_hash],
        bump
    )]
    pub remote_call: Account<'info, RemoteCall>,

    pub output_root: Account<'info, OutputRoot>,

    pub system_program: Program<'info, System>,
}

pub fn prove_call_handler(
    ctx: Context<ProveCall>,
    call_hash: [u8; 32],
    nonce: [u8; 32],
    sender: [u8; 20],
    data: Vec<u8>,
) -> Result<()> {
    require!(
        hash_call(&nonce, &sender, &data) == call_hash,
        ProveCallError::InvalidCallHash
    );

    // TODO: MMR proof verification
    require!(true, ProveCallError::InvalidProof);

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
    #[msg("Invalid call hash")]
    InvalidCallHash,
    #[msg("Invalid proof")]
    InvalidProof,
}
