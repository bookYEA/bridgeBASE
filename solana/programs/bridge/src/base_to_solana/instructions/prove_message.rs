use anchor_lang::{prelude::*, solana_program::keccak};

use crate::base_to_solana::{
    constants::INCOMING_MESSAGE_SEED,
    internal::mmr::{self, Proof},
    state::{IncomingMessage, OutputRoot},
};

#[derive(Accounts)]
pub struct ProveMessage<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + IncomingMessage::INIT_SPACE,
        // // NOTE: We check that the PDA derivation is correct in the handler to optimize the CPI.
        // seeds = [MESSAGE_FROM_BASE_SEED, &message_hash],
        // bump
    )]
    pub message: Account<'info, IncomingMessage>,

    pub output_root: Account<'info, OutputRoot>,

    pub system_program: Program<'info, System>,
}

pub fn prove_message_handler(
    ctx: Context<ProveMessage>,
    nonce: u64,
    sender: [u8; 20],
    data: Vec<u8>,
    proof: Proof,
) -> Result<()> {
    // Hash the message
    let message_hash = hash_message(&nonce.to_le_bytes(), &sender, &data);

    // Verify the PDA derivation is correct
    let (message_pda, _) =
        Pubkey::find_program_address(&[INCOMING_MESSAGE_SEED, &message_hash], ctx.program_id);
    require!(
        message_pda == ctx.accounts.message.key(),
        ProveMessageError::InvalidPda
    );

    // Verify the merkle proof to ensure the transaction exists on the source chain
    mmr::verify_proof(&ctx.accounts.output_root.root, &message_hash, &proof)?;

    ctx.accounts.message.executed = false;
    ctx.accounts.message.sender = sender;
    ctx.accounts.message.data = data;

    Ok(())
}

fn hash_message(nonce: &[u8], sender: &[u8; 20], data: &[u8]) -> [u8; 32] {
    let mut data_to_hash = Vec::new();
    data_to_hash.extend_from_slice(nonce);
    data_to_hash.extend_from_slice(sender);
    data_to_hash.extend_from_slice(data);

    keccak::hash(&data_to_hash).0
}

#[error_code]
pub enum ProveMessageError {
    #[msg("Invalid PDA derivation")]
    InvalidPda,
    #[msg("Invalid proof")]
    InvalidProof,
}
