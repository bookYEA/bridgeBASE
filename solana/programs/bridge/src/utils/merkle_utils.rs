use anchor_lang::solana_program::keccak;

/// Verifies an MMR proof.
///
/// The proof consists of sibling hashes along the path from the leaf to its
/// mountain's peak, followed by the hashes of all other mountain peaks (right-to-left).
///
/// # Arguments
/// * `proof` - The proof elements.
/// * `root` - The expected MMR root.
/// * `leaf_hash` - The hash of the leaf being verified.
/// * `leaf_index` - The 0-indexed position of the leaf in the MMR.
/// * `total_leaf_count` - The total number of leaves in the MMR when the proof was generated.
///
/// # Returns
/// `true` if the proof is valid, `false` otherwise.
pub fn verify_mmr_proof(
    proof: &[[u8; 32]],
    expected_root: &[u8; 32],
    leaf_hash: &[u8; 32],
    leaf_index: u64,
    total_leaf_count: u64,
) -> bool {
    if total_leaf_count == 0 {
        return if proof.is_empty() {
            *expected_root == [0u8; 32] // Or some other representation for an empty root
        } else {
            false
        };
    }
    if leaf_index >= total_leaf_count {
        return false; // Invalid leaf_index
    }

    match calculate_mmr_root_from_proof(proof, leaf_hash, leaf_index, total_leaf_count) {
        Ok(calculated_root) => calculated_root == *expected_root,
        Err(_) => false, // If there was an error in proof processing
    }
}

/// Calculates the MMR root given a leaf, its proof, and the MMR structure.
///
/// This function reconstructs the peaks of the MMR based on the provided leaf and its proof,
/// then bags these peaks together to form the final MMR root.
fn calculate_mmr_root_from_proof(
    proof: &[[u8; 32]],
    leaf_hash: &[u8; 32],
    leaf_idx: u64, // 0-indexed leaf position
    total_leaf_count: u64,
) -> Result<[u8; 32], &'static str> {
    if total_leaf_count == 0 {
        // Consistent with Go: (nil, nil) for empty MMR. Here, maybe a specific error or a zero hash.
        // For now, let's assume an empty MMR root is all zeros or handled by caller.
        return Err("MMR is empty");
    }

    // 1. Determine the mountain structure and the leaf's mountain details.
    let mut mountains: Vec<(u32, u64, bool)> = Vec::new(); // (height, num_leaves_in_mountain, is_leafs_mountain)
    let mut temp_leaf_count = total_leaf_count;
    let mut current_leaf_offset: u64 = 0; // Tracks leaves before the current mountain being considered
    let mut leaf_s_mountain_details: Option<(u32, u64)> = None; // (height, leaf_idx_in_mountain)

    let max_h = if total_leaf_count > 0 {
        64 - total_leaf_count.leading_zeros() - 1
    } else {
        0
    };

    for h_idx in 0..=max_h {
        let h = max_h - h_idx; // Iterate from largest height downwards
        if (temp_leaf_count >> h) & 1 == 1 {
            let leaves_in_this_mountain = 1u64 << h;
            let is_leafs_m = leaf_idx >= current_leaf_offset
                && leaf_idx < current_leaf_offset + leaves_in_this_mountain;
            mountains.push((h, leaves_in_this_mountain, is_leafs_m));
            if is_leafs_m {
                leaf_s_mountain_details = Some((h, leaf_idx - current_leaf_offset));
            }
            current_leaf_offset += leaves_in_this_mountain;
            temp_leaf_count -= leaves_in_this_mountain;
        }
        if temp_leaf_count == 0 {
            break;
        }
    }

    let (leaf_mountain_height, _leaf_idx_in_mountain) = match leaf_s_mountain_details {
        Some(details) => details,
        None => return Err("Leaf's mountain not found - internal logic error"),
    };

    // 2. Calculate the peak of the leaf's mountain.
    let mut current_computed_hash = *leaf_hash;
    let mut proof_idx_offset = 0; // Tracks how many proof elements we've used for intra-mountain

    if leaf_mountain_height as usize > proof.len() && leaf_mountain_height > 0 {
        return Err("Insufficient proof elements for intra-mountain path");
    }

    for _h_climb in 0..leaf_mountain_height {
        let sibling_hash = proof[proof_idx_offset];
        proof_idx_offset += 1;
        current_computed_hash = commutative_keccak256(current_computed_hash, sibling_hash);
    }
    let leaf_mountain_peak_hash = current_computed_hash;

    // 3. Collect all peak hashes (leaf's calculated peak + other peaks from proof).
    let mut all_peak_hashes: Vec<[u8; 32]> = Vec::new();
    let mut remaining_proof_idx = proof_idx_offset;

    // Peaks are needed in right-to-left order for bagging.
    // The `mountains` vector is currently left-to-right.
    for (_height, _num_leaves, is_leafs_m) in mountains.iter().rev() {
        if *is_leafs_m {
            all_peak_hashes.push(leaf_mountain_peak_hash);
        } else {
            // This is an "other" peak, take it from the proof.
            if remaining_proof_idx >= proof.len() {
                return Err("Insufficient proof elements for other mountain peaks");
            }
            all_peak_hashes.push(proof[remaining_proof_idx]);
            remaining_proof_idx += 1;
        }
    }

    if remaining_proof_idx != proof.len() {
        return Err("Unused proof elements remaining");
    }

    // 4. Bag the peaks (right-to-left).
    // `all_peak_hashes` is already in right-to-left mountain order because we iterated `mountains.iter().rev()`.
    if all_peak_hashes.is_empty() {
        // Should be caught by total_leaf_count == 0 earlier, but as a safeguard.
        if total_leaf_count > 0 {
            return Err("No peaks found for non-empty MMR");
        }
        // If total_leaf_count is 0, what should an empty root be? Let's assume [0u8;32]
        return Ok([0u8; 32]);
    }

    let mut current_root = all_peak_hashes[0]; // Start with the rightmost peak.
    for peak_hash in all_peak_hashes.iter().skip(1) {
        // next_peak_hash is to the left of current_root.
        // Hashing order for bagging: H(LeftPeak, H(MiddlePeak, RightPeak))
        // So, current_root is the right operand, all_peak_hashes[i] is the left.
        current_root = commutative_keccak256(*peak_hash, current_root);
    }

    Ok(current_root)
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
