import { keccak256 } from "js-sha3";

export async function deriveRoot(
  batch: number[][]
): Promise<{ root: number[]; proof: number[][] }> {
  if (!batch || batch.length === 0) {
    // The Merkle root of an empty set of leaves is conventionally the hash of an empty input.
    const emptyInput = new Uint8Array(0);
    const hashHex = keccak256(emptyInput);
    const emptyRoot = new Uint8Array(Buffer.from(hashHex, "hex"));
    return { root: [...emptyRoot], proof: [] };
  }

  let currentLevelNodes: Uint8Array[] = batch.map(
    (leaf) => new Uint8Array(leaf)
  );
  const proofElements: Uint8Array[] = [];
  let targetIndexInCurrentLevel = 0; // Tracks the index of the (ancestor of the) first leaf

  while (currentLevelNodes.length > 1) {
    // Determine sibling for the proof for the current targetNodeIndex
    if (targetIndexInCurrentLevel % 2 === 0) {
      // Target is a left node
      if (targetIndexInCurrentLevel + 1 < currentLevelNodes.length) {
        // Has a distinct right sibling
        proofElements.push(currentLevelNodes[targetIndexInCurrentLevel + 1]);
      } else {
        // It's the last node, odd one out, effectively hashed with itself. Sibling is itself.
        proofElements.push(currentLevelNodes[targetIndexInCurrentLevel]);
      }
    } else {
      // Target is a right node, sibling is the left node of the pair
      proofElements.push(currentLevelNodes[targetIndexInCurrentLevel - 1]);
    }

    const nextLevelNodes: Uint8Array[] = [];
    for (let i = 0; i < currentLevelNodes.length; i += 2) {
      const node1 = currentLevelNodes[i];
      // If there's an odd number of nodes, the last node is duplicated and hashed with itself.
      const node2 =
        i + 1 < currentLevelNodes.length ? currentLevelNodes[i + 1] : node1;

      // Sort nodes before concatenation to match commutative hashing onchain
      let firstNode = node1;
      let secondNode = node2;

      // Lexicographical comparison
      let shouldSwap = false;
      const len = Math.min(node1.length, node2.length);
      for (let k = 0; k < len; k++) {
        if (node1[k] > node2[k]) {
          shouldSwap = true;
          break;
        }
        if (node1[k] < node2[k]) {
          break;
        }
      }
      // If one is a prefix of the other, the shorter one comes first if not swapped by content
      if (
        !shouldSwap &&
        node1.length > node2.length &&
        node1
          .slice(0, len)
          .every((val, idx) => val === node2.slice(0, len)[idx])
      ) {
        // This case means node2 is a prefix of node1, but they were equal up to len.
        // To ensure a consistent sort order when one is a prefix of the other (e.g. [1,2] and [1,2,3]),
        // and they are otherwise identical up to the shorter length,
        // the shorter one should come first if we want to replicate typical lexicographical sort.
        // However, given typical Merkle tree implementations with fixed-size hashes, this scenario might be less common
        // or handled by padding. For now, if node1 is longer and they are equal up to min length,
        // they are already in firstNode=node1, secondNode=node2. If node2 was shorter and should come first,
        // it would have been caught by `node1[k] > node2[k]` earlier if there was a difference.
        // The critical part for `a < b` in Rust (for [u8;32]) is direct lexicographical comparison.
        // If node1 = [1,2,3] and node2 = [1,2], node1 > node2.
        if (node1.length > node2.length) shouldSwap = true; // if they were equal up to min(len) and node1 is longer
      }

      if (shouldSwap) {
        firstNode = node2;
        secondNode = node1;
      }

      const concatenated = new Uint8Array(firstNode.length + secondNode.length);
      concatenated.set(firstNode, 0);
      concatenated.set(secondNode, firstNode.length);

      const hashHex = keccak256(concatenated);
      nextLevelNodes.push(new Uint8Array(Buffer.from(hashHex, "hex")));
    }
    currentLevelNodes = nextLevelNodes;
    targetIndexInCurrentLevel = Math.floor(targetIndexInCurrentLevel / 2); // Update index for the next level
  }

  // The single remaining node is the Merkle root.
  const rootHash = currentLevelNodes[0];
  const finalRoot = Array.from(rootHash);
  const finalProof = proofElements.map((p) => Array.from(p));

  return { root: finalRoot, proof: finalProof };
}
