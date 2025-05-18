use anchor_lang::solana_program::keccak;

/**
 * @dev Returns true if a `leaf` can be proved to be a part of a Merkle tree
 * defined by `root`. For this, a `proof` must be provided, containing
 * sibling hashes on the branch from the leaf to the root of the tree. Each
 * pair of leaves and each pair of pre-images are assumed to be sorted.
 *
 * This version handles proofs in memory with the default hashing function.
 */
pub fn verify(proof: &[[u8; 32]], root: &[u8; 32], leaf: &[u8; 32]) -> bool {
    process_proof(proof, leaf) == *root
}

/**
 * @dev Returns the rebuilt hash obtained by traversing a Merkle tree up
 * from `leaf` using `proof`. A `proof` is valid if and only if the rebuilt
 * hash matches the root of the tree. When processing the proof, the pairs
 * of leaves & pre-images are assumed to be sorted.
 *
 * This version handles proofs in memory with the default hashing function.
 */
fn process_proof(proof: &[[u8; 32]], leaf: &[u8; 32]) -> [u8; 32] {
    let mut computed_hash = *leaf;

    for node in proof {
        computed_hash = commutative_keccak256(computed_hash, *node);
    }

    computed_hash
}

/**
 * @dev Commutative Keccak256 hash of a sorted pair of bytes32. Frequently used when working with merkle proofs.
 *
 * NOTE: Equivalent to the `standardNodeHash` in our https://github.com/OpenZeppelin/merkle-tree[JavaScript library].
 */
fn commutative_keccak256(a: [u8; 32], b: [u8; 32]) -> [u8; 32] {
    if a < b {
        efficient_keccak256(&a, &b)
    } else {
        efficient_keccak256(&b, &a)
    }
}

/**
 * @dev Implementation of keccak256(abi.encode(a, b)) that doesn't allocate or expand memory.
 */
fn efficient_keccak256(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    let mut data_to_hash = Vec::new();
    data_to_hash.extend_from_slice(a);
    data_to_hash.extend_from_slice(b);
    keccak::hash(&data_to_hash).to_bytes()
}
