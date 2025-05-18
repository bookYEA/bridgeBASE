use crate::Ix;
use anchor_lang::solana_program::keccak;

/// Creates a hash of the instructions to identify the transaction.
pub fn hash_ixs(remote_sender: &[u8; 20], ixs: &[Ix]) -> [u8; 32] {
    // Create a canonical representation of the instructions.
    let mut data = Vec::new();

    data.extend_from_slice(remote_sender);

    // Add each instruction.
    for ix in ixs {
        // Add program ID.
        data.extend_from_slice(&ix.program_id.to_bytes());

        // Add each account.
        for account in &ix.accounts {
            data.extend_from_slice(&account.pubkey.to_bytes());
            data.push(account.is_writable as u8);
            data.push(account.is_signer as u8);
        }

        // Add data.
        data.extend_from_slice(&ix.data);
    }

    // Hash the data using keccak256.
    keccak::hash(&data).0
}
