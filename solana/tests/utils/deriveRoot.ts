import { keccak256 } from "js-sha3";

// Helper to convert a number array (bytes) to Uint8Array
function toUint8Array(arr: number[]): Uint8Array {
  return new Uint8Array(arr);
}

// Helper to convert Uint8Array to number array
function toNumberArray(arr: Uint8Array): number[] {
  return Array.from(arr);
}

// Commutative Keccak256 hash function (matches Go and Rust impl)
function internalHash(node1: Uint8Array, node2: Uint8Array): Uint8Array {
  let firstNode = node1;
  let secondNode = node2;

  // Lexicographical comparison for sorting
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
  if (!shouldSwap && node1.length !== node2.length) {
    if (node1.length > node2.length) {
      // If node1 is longer and they were equal up to min(len), node1 is "greater"
      // This handles cases like [1,2,3] vs [1,2]
      let isPrefix = true;
      for (let k = 0; k < node2.length; ++k) {
        if (node1[k] !== node2[k]) {
          isPrefix = false;
          break;
        }
      }
      if (isPrefix) shouldSwap = true;
    }
    // else node2 is longer, node1 is already first, no swap needed if it was a prefix
  }

  if (shouldSwap) {
    firstNode = node2;
    secondNode = node1;
  }

  const concatenated = new Uint8Array(firstNode.length + secondNode.length);
  concatenated.set(firstNode, 0);
  concatenated.set(secondNode, firstNode.length);

  const hashHex = keccak256(concatenated);
  return new Uint8Array(Buffer.from(hashHex, "hex"));
}

/**
 * Simulates the Go MMR's node construction and returns all nodes.
 * The order of nodes in this list corresponds to their canonical construction order.
 */
function buildMMRNodes(leafHashes: Uint8Array[]): Uint8Array[] {
  if (leafHashes.length === 0) {
    return [];
  }

  const allNodes: Uint8Array[] = [];
  let currentLeafCount = 0;

  for (const leafH of leafHashes) {
    allNodes.push(leafH);
    const originalLeafCountBeforeAppend = currentLeafCount;
    currentLeafCount++;

    let currentNodeIdx = allNodes.length - 1;
    let currentHeight = 0;

    while (((originalLeafCountBeforeAppend >> currentHeight) & 1) === 1) {
      const leftSubtreeSize = (1 << (currentHeight + 1)) - 1;
      const leftSiblingNodeIdx = currentNodeIdx - leftSubtreeSize;

      if (leftSiblingNodeIdx < 0 || leftSiblingNodeIdx >= allNodes.length) {
        throw new Error(
          `Internal MMR error: invalid left sibling index ${leftSiblingNodeIdx} for current node ${currentNodeIdx} at height ${currentHeight}`
        );
      }

      const leftNodeHash = allNodes[leftSiblingNodeIdx];
      const rightNodeHash = allNodes[currentNodeIdx];

      const parentNodeHash = internalHash(leftNodeHash, rightNodeHash);
      allNodes.push(parentNodeHash);

      currentNodeIdx = allNodes.length - 1;
      currentHeight++;
    }
  }
  return allNodes;
}

/**
 * Gets the node indices of the peaks for the current leafCount.
 * Mimics Go's getPeakNodeIndices, returning peaks ordered right-to-left.
 */
function getPeakNodePositions(
  leafCount: number,
  numAllNodes: number
): number[] {
  if (leafCount === 0) {
    return [];
  }

  const peakIndices: number[] = [];
  let tempLeafCount = leafCount;
  let accumulatedNodeOffset = 0;

  const maxPossibleHeight = Math.floor(Math.log2(leafCount));

  for (let h = maxPossibleHeight; h >= 0; h--) {
    if ((tempLeafCount >> h) & 1) {
      const numNodesInThisMountain = (1 << (h + 1)) - 1;
      const peakIdx = accumulatedNodeOffset + numNodesInThisMountain - 1;

      if (peakIdx >= numAllNodes) {
        throw new Error(
          `Internal MMR error: calculated peak index ${peakIdx} out of bounds ${numAllNodes}`
        );
      }
      peakIndices.push(peakIdx);
      accumulatedNodeOffset += numNodesInThisMountain;
      tempLeafCount -= 1 << h;
    }
    if (tempLeafCount === 0 && h > 0) break; // Optimization from Go, if all leaves accounted for
  }

  return peakIndices.reverse(); // Go reverses, so we match (right-to-left peaks)
}

export async function deriveRoot(batch: number[][]): Promise<{
  root: number[];
  proof: number[][];
  leafIndexForProof: number; // leaf index for which the proof is generated
  totalLeaves: number; // total leaves in this MMR
}> {
  const leafHashes = batch.map(toUint8Array);
  const totalLeaves = leafHashes.length;

  if (totalLeaves === 0) {
    const emptyRoot = internalHash(new Uint8Array(0), new Uint8Array(0)); // Or some defined empty hash
    return {
      root: toNumberArray(emptyRoot),
      proof: [],
      leafIndexForProof: 0,
      totalLeaves: 0,
    };
  }

  const allMMRNodes = buildMMRNodes(leafHashes);

  // --- Calculate Root ---
  const peakNodePositionsRoot = getPeakNodePositions(
    totalLeaves,
    allMMRNodes.length
  );
  if (peakNodePositionsRoot.length === 0 && totalLeaves > 0) {
    throw new Error("No peaks found for non-empty MMR during root calculation");
  }

  let mmrRoot: Uint8Array;
  if (totalLeaves === 0) {
    mmrRoot = internalHash(new Uint8Array(0), new Uint8Array(0)); // Placeholder for empty
  } else if (peakNodePositionsRoot.length === 0 && totalLeaves > 0) {
    throw new Error("No peaks for root calc");
  } else if (peakNodePositionsRoot.length === 0 && totalLeaves === 0) {
    mmrRoot = internalHash(new Uint8Array(0), new Uint8Array(0));
  } else {
    mmrRoot = allMMRNodes[peakNodePositionsRoot[0]];
    for (let i = 1; i < peakNodePositionsRoot.length; i++) {
      const nextPeakHash = allMMRNodes[peakNodePositionsRoot[i]];
      mmrRoot = internalHash(nextPeakHash, mmrRoot); // Bagging: H(Left, Right)
    }
  }

  // --- Generate Proof (Simplified for the first leaf, index 0) ---
  const leafIndexForProof = 0;
  const proofElements: Uint8Array[] = [];

  // 1. Find information about the mountain containing the target leaf (leaf 0)
  let actualMountainHeight = 0;
  let actualLeafIdxInMountain = 0; // For leaf 0, this is 0 if it's in the first mountain
  let actualMountainPeakPos = -1;
  let foundMountain = false;

  let tempOverallLeafCount_proof = totalLeaves;
  let accumulatedNodes_proof = 0;
  let accumulatedLeaves_proof = 0;
  const maxPossibleH_proof =
    totalLeaves > 0 ? Math.floor(Math.log2(totalLeaves)) : 0;

  for (let h = maxPossibleH_proof; h >= 0; h--) {
    if ((tempOverallLeafCount_proof >> h) & 1) {
      const leavesInThisMountain = 1 << h;
      const nodesInThisMountain = (1 << (h + 1)) - 1;
      if (
        leafIndexForProof >= accumulatedLeaves_proof &&
        leafIndexForProof < accumulatedLeaves_proof + leavesInThisMountain
      ) {
        actualMountainHeight = h;
        actualLeafIdxInMountain = leafIndexForProof - accumulatedLeaves_proof;
        actualMountainPeakPos =
          accumulatedNodes_proof + nodesInThisMountain - 1;
        foundMountain = true;
        break;
      }
      accumulatedNodes_proof += nodesInThisMountain;
      accumulatedLeaves_proof += leavesInThisMountain;
      tempOverallLeafCount_proof -= leavesInThisMountain;
    }
    if (tempOverallLeafCount_proof === 0 && h > 0) break;
  }

  if (!foundMountain) {
    throw new Error(
      `Could not locate mountain for leafIndex ${leafIndexForProof}`
    );
  }

  // 2. Calculate intra-mountain proof for leaf 0
  // Need its actual node position. For leaf 0, its node position is 0 in a simple MMR start.
  // This part needs careful mapping from logical leaf index to physical node position.
  // The `leafIndexToNodePosition` logic from Go is complex.
  // For leaf 0 in the first mountain, its node position is simply 0.
  // If MMR nodes are built correctly, allMMRNodes[0] is the first leaf.
  let currentPathNodePos = -1; // This needs to be the *MMR node index* of the leaf.

  // Simplified leafIndexToNodePosition for leafIndex = 0:
  // It's more complex than this in general. This assumes leaf 0 is node 0 in allMMRNodes.
  if (leafIndexForProof === 0 && allMMRNodes.length > 0) {
    currentPathNodePos = 0;
  } else {
    // A proper leafIndexToNodePosition would be needed here for other leaves.
    // This is a placeholder calculation that will likely be incorrect for leafIndex > 0 or complex MMRs.
    // The Go function `leafIndexToNodePosition` is what's truly needed here.
    // For now, we assume leafIndex 0 maps to allMMRNodes[0].
    if (allMMRNodes.length > leafIndexForProof) {
      currentPathNodePos = leafIndexForProof; // This is a common pattern if leaves are added sequentially at the start
    } else {
      throw new Error(
        "Cannot determine node position for proof generation for leaf " +
          leafIndexForProof
      );
    }
  }
  if (currentPathNodePos === -1 || currentPathNodePos >= allMMRNodes.length) {
    throw new Error(
      `Could not get node position for leaf ${leafIndexForProof}`
    );
  }

  for (let hClimb = 0; hClimb < actualMountainHeight; hClimb++) {
    const isRightChildInSubtree = (actualLeafIdxInMountain >> hClimb) & 1;
    let siblingNodePos = -1;
    let parentNodePos = -1;

    if (isRightChildInSubtree) {
      parentNodePos = currentPathNodePos + 1;
      siblingNodePos = parentNodePos - (1 << (hClimb + 1));
    } else {
      parentNodePos = currentPathNodePos + (1 << (hClimb + 1));
      siblingNodePos = parentNodePos - 1;
    }

    if (siblingNodePos < 0 || siblingNodePos >= allMMRNodes.length) {
      throw new Error(`Invalid sibling index ${siblingNodePos} for proof`);
    }
    proofElements.push(allMMRNodes[siblingNodePos]);
    currentPathNodePos = parentNodePos;
    if (currentPathNodePos < 0 || currentPathNodePos >= allMMRNodes.length) {
      throw new Error(
        `Invalid parent node position ${parentNodePos} during proof climb`
      );
    }
  }
  if (currentPathNodePos !== actualMountainPeakPos) {
    throw new Error(
      `Path climbing did not reach mountain peak. Reached ${currentPathNodePos}, expected ${actualMountainPeakPos}`
    );
  }

  // 3. Add hashes of other mountain peaks
  const allPeakNodePositionsProof = getPeakNodePositions(
    totalLeaves,
    allMMRNodes.length
  );
  for (const peakPos of allPeakNodePositionsProof) {
    if (peakPos !== actualMountainPeakPos) {
      proofElements.push(allMMRNodes[peakPos]);
    }
  }

  return {
    root: toNumberArray(mmrRoot),
    proof: proofElements.map(toNumberArray),
    leafIndexForProof,
    totalLeaves,
  };
}
