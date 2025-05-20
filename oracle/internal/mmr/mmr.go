package mmr

import (
	"crypto/sha256"
	"fmt"
	"math/bits" // For bits.Len64
)

// Hash represents a hash value in the MMR.
type Hash []byte

// NodePosition represents the 0-indexed position of a node in the MMR's flat list of nodes.
type NodePosition uint64

// MMR holds the state of the Merkle Mountain Range.
// It stores all nodes (leaves and internal nodes) in a flat list `nodes`.
// The order of nodes in this list corresponds to their canonical construction order:
// e.g., L0, L1, P(L0,L1), L2, L3, P(L2,L3), P(P(L0,L1),P(L2,L3)), ...
type MMR struct {
	nodes     []Hash // All nodes (leaves and internal) in the MMR.
	leafCount uint64 // Number of leaves added to the MMR.
}

// NewMMR creates a new, empty MMR.
func NewMMR() *MMR {
	return &MMR{
		nodes:     make([]Hash, 0),
		leafCount: 0,
	}
}

// internalHash concatenates and hashes two node hashes to form a parent hash.
// The order is H(leftNode, rightNode).
func internalHash(left Hash, right Hash) Hash {
	h := sha256.New()
	h.Write(left)
	h.Write(right)
	return h.Sum(nil)
}

// Append adds new data to the MMR.
// It calculates the leaf hash, adds it to the list of nodes,
// and then computes and adds any new parent nodes that can be formed due to this addition.
// It returns the MMR position (index in the internal nodes list) of the added leaf.
func (m *MMR) Append(leafH Hash) (NodePosition, error) {
	m.nodes = append(m.nodes, leafH)
	originalLeafCountBeforeAppend := m.leafCount
	m.leafCount++

	// The MMR position of the leaf we just added.
	addedLeafNodeIndex := len(m.nodes) - 1
	leafMMRPosition := NodePosition(addedLeafNodeIndex)

	// This part forms parent nodes.
	// `currentNodeIdx` refers to the index of the right-hand node of a potential merge.
	// Initially, it's the leaf we just added. If a parent is formed, `currentNodeIdx` becomes that parent.
	currentNodeIdx := addedLeafNodeIndex
	// `currentHeight` is the height of the subtrees being merged.
	// e.g., two H=0 leaves/subtrees merge to form an H=1 parent.
	currentHeight := 0

	// Loop to create parent nodes. This loop continues as long as the previously existing
	// mountain structure (represented by `originalLeafCountBeforeAppend`) had a peak
	// at `currentHeight` that can now be merged with the newly added branch (which has also reached `currentHeight`).
	// The condition `(originalLeafCountBeforeAppend >> uint(currentHeight))&1 == 1` checks if the
	// (currentHeight)-th bit of `originalLeafCountBeforeAppend` is set. If so, a merge is possible.
	for (originalLeafCountBeforeAppend>>uint(currentHeight))&1 == 1 {
		// The node at `m.nodes[currentNodeIdx]` is the root of the right subtree of height `currentHeight`.
		// We need to find its left sibling, which is the root of the left subtree of height `currentHeight`.
		// The number of nodes in a full binary tree of height `currentHeight` is (2^(currentHeight+1) - 1).
		// The left sibling node is located at `currentNodeIdx - size_of_full_tree(currentHeight)`.
		leftSubtreeSize := (uint64(1) << (uint(currentHeight) + 1)) - 1
		leftSiblingNodeIdx := uint64(currentNodeIdx) - leftSubtreeSize

		if leftSiblingNodeIdx >= uint64(len(m.nodes)) { // Check for underflow or invalid index
			return leafMMRPosition, fmt.Errorf("internal MMR error: invalid left sibling index %d for current node %d at height %d", leftSiblingNodeIdx, currentNodeIdx, currentHeight)
		}

		leftNodeHash := m.nodes[leftSiblingNodeIdx]
		rightNodeHash := m.nodes[currentNodeIdx] // This is the hash of the node we are "promoting" (root of right subtree)

		parentNodeHash := internalHash(leftNodeHash, rightNodeHash)
		m.nodes = append(m.nodes, parentNodeHash)

		currentNodeIdx = len(m.nodes) - 1 // Parent becomes the new current node for the next iteration.
		currentHeight++                   // The height of the newly formed parent is one greater.
	}

	return leafMMRPosition, nil
}

// Size returns the number of leaves in the MMR.
func (m *MMR) Size() uint64 {
	return m.leafCount
}

// IsEmpty returns true if the MMR has no leaves.
func (m *MMR) IsEmpty() bool {
	return m.leafCount == 0
}

// getPeakNodeIndices returns the MMR node indices of the peaks for the current leafCount.
// The returned peak indices are ordered from rightmost (smallest height, last in bagging)
// to leftmost (largest height, first in bagging sequence if H(L, H(M,R))).
func (m *MMR) getPeakNodeIndices() ([]NodePosition, error) {
	if m.leafCount == 0 {
		return []NodePosition{}, nil
	}

	var peakIndices []NodePosition
	var tempLeafCount = m.leafCount      // Temporary leaf counter to identify peaks
	var accumulatedNodeOffset uint64 = 0 // Tracks the number of nodes in mountains to the left of current one

	maxPossibleHeight := uint(0)
	if m.leafCount > 0 {
		maxPossibleHeight = uint(bits.Len64(m.leafCount) - 1)
	}

	// Iterate from the largest possible mountain height downwards to identify peaks from left to right.
	for h := maxPossibleHeight; ; h-- {
		// Check if a mountain of this height `h` exists at the current position.
		// This is true if the h-th bit of `tempLeafCount` is set.
		if (tempLeafCount>>h)&1 == 1 {
			// This mountain contributes (1 << h) leaves.
			// The number of nodes in this mountain (a perfect binary tree of height h) is (1 << (h+1)) - 1.
			numNodesInThisMountain := (uint64(1) << (h + 1)) - 1

			// The root of this mountain is the last node in its canonical sequence.
			// Its index in m.nodes is (accumulatedNodeOffset + numNodesInThisMountain - 1).
			peakIdx := accumulatedNodeOffset + numNodesInThisMountain - 1
			if peakIdx >= uint64(len(m.nodes)) {
				return nil, fmt.Errorf("internal MMR error: calculated peak index %d out of bounds %d for height %d, leafCount %d (temp: %d)", peakIdx, len(m.nodes), h, m.leafCount, tempLeafCount)
			}
			peakIndices = append(peakIndices, NodePosition(peakIdx))

			accumulatedNodeOffset += numNodesInThisMountain
			tempLeafCount -= (uint64(1) << h) // Account for leaves covered by this mountain
		}
		if h == 0 { // Avoid underflow for unsigned h and ensure loop termination
			break
		}
	}

	// The list of peaks was generated from left (largest mountain) to right (smallest).
	// For standard right-to-left bagging (e.g. H(LeftPeak, H(MiddlePeak, RightPeak))),
	// we need to reverse this order so the rightmost peak is first in the list.
	for i, j := 0, len(peakIndices)-1; i < j; i, j = i+1, j-1 {
		peakIndices[i], peakIndices[j] = peakIndices[j], peakIndices[i]
	}

	return peakIndices, nil
}

// Root calculates the single Merkle root of the MMR by "bagging the peaks".
// Returns (nil, nil) if the MMR is empty.
func (m *MMR) Root() (Hash, error) {
	if m.IsEmpty() {
		return nil, nil
	}

	peakNodeIdxs, err := m.getPeakNodeIndices()
	if err != nil {
		return nil, fmt.Errorf("failed to get peak indices for root calculation: %w", err)
	}

	if len(peakNodeIdxs) == 0 {
		// This should ideally not happen if leafCount > 0, covered by IsEmpty check.
		// If leafCount > 0 and no peaks, it's an internal error.
		return nil, fmt.Errorf("internal MMR error: no peaks found for non-empty MMR (leaves: %d)", m.leafCount)
	}

	// Bag the peaks. Peaks are ordered right-to-left (e.g., [R, M, L]).
	// Bagging: H(L, H(M,R))
	// Start with the rightmost peak (first in our reversed list).
	currentRoot := m.nodes[peakNodeIdxs[0]]

	// Sequentially hash with the next peak to the left.
	// `currentRoot` is the right operand, `nextPeakHash` (to its left) is the left operand.
	for i := 1; i < len(peakNodeIdxs); i++ {
		nextPeakHash := m.nodes[peakNodeIdxs[i]]
		currentRoot = internalHash(nextPeakHash, currentRoot)
	}

	return currentRoot, nil
}

// leafIndexToNodePosition converts a 0-indexed logical leaf number to its
// MMR node position (its index in the m.nodes array).
// This is crucial for fetching a leaf's hash or constructing a proof for a specific leaf.
func (m *MMR) leafIndexToNodePosition(leafIndex uint64) (NodePosition, error) {
	if leafIndex >= m.leafCount {
		return 0, fmt.Errorf("leafIndex %d is out of bounds for current leafCount %d", leafIndex, m.leafCount)
	}

	var leafIdxToFind = leafIndex              // The target leaf index we are counting towards.
	var nodesBeforeCurrentMountain uint64 = 0  // Offset for node indices due to mountains to the left.
	var leavesBeforeCurrentMountain uint64 = 0 // Offset for leaf indices.

	// Determine the structure of mountains based on the total leafCount.
	// Iterate from largest possible mountain height downwards.
	maxPossibleHeight := uint(0)
	if m.leafCount > 0 {
		maxPossibleHeight = uint(bits.Len64(m.leafCount) - 1)
	}

	effectiveLeafCount := m.leafCount // Used to check the structure of peaks

	for h := maxPossibleHeight; ; h-- {
		// Check if a mountain of height `h` is the next one in the sequence.
		// This is true if the h-th bit of the remaining leaf count (after accounting for larger mountains) is set.
		if ((effectiveLeafCount-leavesBeforeCurrentMountain)>>h)&1 == 1 {
			leavesInThisMountain := uint64(1) << h

			if leafIdxToFind < leavesInThisMountain {
				// The target leaf is in this current mountain.
				// Leaves within this mountain are indexed 0 to (leavesInThisMountain-1) locally.
				// Their actual MMR node positions are: nodesBeforeCurrentMountain + local_leaf_index.
				// The local_leaf_index for our target is leafIdxToFind.
				return NodePosition(nodesBeforeCurrentMountain + leafIdxToFind), nil
			}

			// The target leaf is not in this mountain; skip past this mountain.
			nodesInThisMountain := (uint64(1) << (h + 1)) - 1
			nodesBeforeCurrentMountain += nodesInThisMountain
			leavesBeforeCurrentMountain += leavesInThisMountain
			leafIdxToFind -= leavesInThisMountain
		}
		if h == 0 {
			break // Avoid underflow for unsigned h
		}
	}

	// If loop completes without finding, it's an internal logic error or inconsistent state.
	return 0, fmt.Errorf("internal MMR error: failed to locate leafIndex %d in node structure (final leafIdxToFind: %d)", leafIndex, leafIdxToFind)
}
