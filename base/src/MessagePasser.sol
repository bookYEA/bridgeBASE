// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

/// @title MessagePasser
///
/// @notice The MessagePasser is a dedicated contract for initiating withdrawals to Solana. Messages sent through this
///         contract contain Solana instructions that will be executed on the Solana network.
contract MessagePasser {
    //////////////////////////////////////////////////////////////
    ///                       Constants                        ///
    //////////////////////////////////////////////////////////////

    /// @notice Maximum number of peaks possible in the MMR
    uint256 private constant MAX_PEAKS = 64;

    //////////////////////////////////////////////////////////////
    ///                       Structs                          ///
    //////////////////////////////////////////////////////////////

    /// @notice Struct representing a remote call to a Solana program.
    ///
    /// @custom:field nonce Unique identifier for this remote call.
    /// @custom:field sender Ethereum address that initiated the remote call.
    /// @custom:field data Data to be passed to the Solana program.
    struct RemoteCall {
        uint64 nonce;
        address sender;
        bytes data;
    }

    //////////////////////////////////////////////////////////////
    ///                       Events                           ///
    //////////////////////////////////////////////////////////////

    /// @notice Emitted when a remote call is initiated.
    ///
    /// @param nonce          Unique nonce for this remote call.
    /// @param sender         The Ethereum address that initiated the remote call.
    /// @param data           Data to be passed to the Solana program.
    /// @param remoteCallHash The hash of the complete remote call.
    event RemoteCallSent(
        uint64 indexed nonce, address indexed sender, bytes data, bytes32 remoteCallHash, bytes32 newRoot
    );

    //////////////////////////////////////////////////////////////
    ///                       Errors                           ///
    //////////////////////////////////////////////////////////////

    /// @notice Thrown when a node query is received with an out of bounds index
    error InvalidIndex();

    /// @notice Thrown when trying to generate a proof for an empty MMR
    error EmptyMMR();

    /// @notice Thrown when the leaf index is out of bounds
    error LeafIndexOutOfBounds();

    /// @notice Thrown when failing to locate a leaf in the MMR structure
    error LeafNotFound();

    /// @notice Thrown when failing to locate the mountain containing a leaf
    error MountainNotFound();

    /// @notice Thrown when a sibling node index is out of bounds
    error SiblingNodeOutOfBounds();

    /// @notice Thrown when a peak index is out of bounds
    error PeakIndexOutOfBounds();

    //////////////////////////////////////////////////////////////
    ///                       Storage                          ///
    //////////////////////////////////////////////////////////////

    /// @notice The current root hash
    bytes32 private _root;

    /// @notice All nodes (leaves and internal) in the MMR
    bytes32[] private _nodes;

    /// @notice Internal counter for generating unique nonces for each remote call.
    uint64 internal _remoteCallNonce;

    //////////////////////////////////////////////////////////////
    ///                  External Functions                    ///
    //////////////////////////////////////////////////////////////

    /// @notice Sends a remote call to a Solana program. This function creates a remote call, hashes it for
    ///         verification, and emits an event that can be monitored by offchain relayers.
    ///
    /// @param data Data to be passed to the Solana program.
    function sendRemoteCall(bytes calldata data) external {
        uint64 currentNonce = _remoteCallNonce;

        bytes32 remoteCallHash = _hashRemoteCall(RemoteCall({nonce: currentNonce, sender: msg.sender, data: data}));
        bytes32 newRoot = _appendLeafToMMR({leafHash: remoteCallHash, originalLeafCount: currentNonce});

        emit RemoteCallSent(currentNonce, msg.sender, data, remoteCallHash, newRoot);

        unchecked {
            ++_remoteCallNonce;
        }
    }

    /// @notice Get the current root of the MMR
    function getRoot() external view returns (bytes32) {
        return _root;
    }

    /// @notice Get the number of leaves in the MMR
    function getLeafCount() external view returns (uint64) {
        return _remoteCallNonce;
    }

    /// @notice Get the total number of nodes in the MMR
    function getNodeCount() external view returns (uint256) {
        return _nodes.length;
    }

    /// @notice Check if the MMR is empty
    function isEmpty() external view returns (bool) {
        return _remoteCallNonce == 0;
    }

    /// @notice Get a node from the MMR
    ///
    /// @param index The index of the node in the nodes array of the MMR
    ///
    /// @return node The node in the MMR
    function getNode(uint256 index) external view returns (bytes32) {
        if (index >= _nodes.length) {
            revert InvalidIndex();
        }
        return _nodes[index];
    }

    /// @notice Generates a Merkle proof for a specific leaf in the MMR
    /// @dev This function may consume significant gas for large MMRs (O(log N) storage reads)
    /// @param leafIndex The 0-indexed position of the leaf to prove
    /// @return proof Array of sibling hashes for the proof
    /// @return totalLeafCount The total number of leaves when proof was generated
    function generateProof(uint64 leafIndex) external view returns (bytes32[] memory proof, uint64 totalLeafCount) {
        if (_remoteCallNonce == 0) {
            revert EmptyMMR();
        }
        if (leafIndex >= _remoteCallNonce) {
            revert LeafIndexOutOfBounds();
        }

        // Use optimized single-pass algorithm
        (
            uint256 leafNodePos,
            uint256 mountainHeight,
            uint64 leafIdxInMountain,
            bytes32[] memory otherPeaks
        ) = _generateProofData(leafIndex);
        
        // Generate intra-mountain proof directly
        bytes32[] memory intraMountainProof = new bytes32[](mountainHeight);
        uint256 currentPathNodePos = leafNodePos;
        
        for (uint256 hClimb = 0; hClimb < mountainHeight; hClimb++) {
            bool isRightChildInSubtree = (leafIdxInMountain >> hClimb) & 1 == 1;
            
            uint256 siblingNodePos;
            uint256 parentNodePos;

            if (isRightChildInSubtree) {
                parentNodePos = currentPathNodePos + 1;
                siblingNodePos = parentNodePos - (1 << (hClimb + 1));
            } else {
                parentNodePos = currentPathNodePos + (1 << (hClimb + 1));
                siblingNodePos = parentNodePos - 1;
            }

            if (siblingNodePos >= _nodes.length) {
                revert SiblingNodeOutOfBounds();
            }

            intraMountainProof[hClimb] = _nodes[siblingNodePos];
            currentPathNodePos = parentNodePos;
        }
        
        // Combine proof elements
        proof = new bytes32[](intraMountainProof.length + otherPeaks.length);
        uint256 proofIndex = 0;
        
        for (uint256 i = 0; i < intraMountainProof.length; i++) {
            proof[proofIndex++] = intraMountainProof[i];
        }
        
        for (uint256 i = 0; i < otherPeaks.length; i++) {
            proof[proofIndex++] = otherPeaks[i];
        }
        
        totalLeafCount = _remoteCallNonce;
    }

    //////////////////////////////////////////////////////////////
    ///                       Internal Functions               ///
    //////////////////////////////////////////////////////////////

    /// @notice Computes the hash of a remote call.
    ///
    /// @param remoteCall The remote call to hash.
    ///
    /// @return hash The keccak256 hash of the encoded remote call.
    function _hashRemoteCall(RemoteCall memory remoteCall) internal pure returns (bytes32) {
        return keccak256(abi.encodePacked(remoteCall.nonce, remoteCall.sender, remoteCall.data));
    }

    //////////////////////////////////////////////////////////////
    ///                     Private Functions                  ///
    //////////////////////////////////////////////////////////////

    /// @dev Append a new leaf to the MMR
    ///
    /// @param leafHash          The hash of the leaf to append
    /// @param originalLeafCount The amount of MMR leaves before the append
    ///
    /// @return newRoot The new root of the MMR after the append is complete
    function _appendLeafToMMR(bytes32 leafHash, uint64 originalLeafCount) private returns (bytes32) {
        // Add the leaf to the nodes array
        _nodes.push(leafHash);

        // The MMR position of the leaf we just added
        uint256 newLeafNodeIndex = _nodes.length - 1;

        // Form parent nodes by merging when possible
        _createParentNodes(newLeafNodeIndex, originalLeafCount);

        // Update and return the new root
        bytes32 newRoot = _calculateRoot();
        _root = newRoot;
        return newRoot;
    }

    /// @dev Create parent nodes by merging when the binary representation allows it
    ///
    /// @param leafNodeIndex The index of the newly added leaf node
    /// @param leafCount The original leaf count before adding the new leaf
    function _createParentNodes(uint256 leafNodeIndex, uint64 leafCount) private {
        uint256 currentNodeIndex = leafNodeIndex;
        uint256 currentHeight = 0;

        // Loop to create parent nodes when merging is possible
        while (_shouldMergeAtHeight(leafCount, currentHeight)) {
            uint256 leftSiblingIndex = _calculateLeftSiblingIndex(currentNodeIndex, currentHeight);

            // Get the hashes to merge
            bytes32 leftNodeHash = _nodes[leftSiblingIndex];
            bytes32 rightNodeHash = _nodes[currentNodeIndex];

            // Create and store the parent node
            bytes32 parentNodeHash = _hashInternalNode(leftNodeHash, rightNodeHash);
            _nodes.push(parentNodeHash);

            // Update for next iteration
            currentNodeIndex = _nodes.length - 1;
            currentHeight++;
        }
    }

    /// @dev Optimized single traversal to get leaf position and other peaks
    /// @param leafIndex The 0-indexed position of the leaf to prove
    /// @return leafNodePos Position of the leaf in the _nodes array
    /// @return mountainHeight Height of the mountain containing the leaf
    /// @return leafIdxInMountain Position of leaf within its mountain
    /// @return otherPeaks Hashes of other mountain peaks
    function _generateProofData(uint64 leafIndex) private view returns (
        uint256 leafNodePos,
        uint256 mountainHeight,
        uint64 leafIdxInMountain,
        bytes32[] memory otherPeaks
    ) {
        // First pass: find the leaf mountain
        (leafNodePos, mountainHeight, leafIdxInMountain) = _findLeafMountain(leafIndex);
        
        // Second pass: collect other peaks
        otherPeaks = _collectOtherPeaks(leafIndex);
    }

    /// @dev Find leaf mountain with minimal local variables
    function _findLeafMountain(uint64 leafIndex) private view returns (uint256, uint256, uint64) {
        uint256 nodeOffset = 0;
        uint64 leafOffset = 0;
        uint256 maxHeight = _calculateMaxPossibleHeight(_remoteCallNonce);

        for (uint256 h = maxHeight + 1; h > 0; h--) {
            uint256 height = h - 1;
            
            if ((_remoteCallNonce >> height) & 1 == 1) {
                uint64 mountainLeaves = uint64(1 << height);
                
                if (leafIndex >= leafOffset && leafIndex < leafOffset + mountainLeaves) {
                    // Found the mountain
                    uint64 localLeafIdx = leafIndex - leafOffset;
                    uint256 localNodePos = 2 * uint256(localLeafIdx) - _popcount(localLeafIdx);
                    return (nodeOffset + localNodePos, height, localLeafIdx);
                }
                
                nodeOffset += (1 << (height + 1)) - 1;
                leafOffset += mountainLeaves;
            }
        }
        
        revert LeafNotFound();
    }

    /// @dev Collect other mountain peaks
    function _collectOtherPeaks(uint64 leafIndex) private view returns (bytes32[] memory) {
        bytes32[] memory tempPeaks = new bytes32[](MAX_PEAKS);
        uint256 peakCount = 0;
        uint256 nodeOffset = 0;
        uint64 leafOffset = 0;
        uint256 maxHeight = _calculateMaxPossibleHeight(_remoteCallNonce);

        for (uint256 h = maxHeight + 1; h > 0; h--) {
            uint256 height = h - 1;
            
            if ((_remoteCallNonce >> height) & 1 == 1) {
                uint64 mountainLeaves = uint64(1 << height);
                bool isLeafMountain = (leafIndex >= leafOffset && leafIndex < leafOffset + mountainLeaves);
                
                if (!isLeafMountain) {
                    uint256 peakPos = nodeOffset + (1 << (height + 1)) - 2;
                    tempPeaks[peakCount++] = _nodes[peakPos];
                }
                
                nodeOffset += (1 << (height + 1)) - 1;
                leafOffset += mountainLeaves;
            }
        }
        
        // Copy to exact size array
        bytes32[] memory peaks = new bytes32[](peakCount);
        for (uint256 i = 0; i < peakCount; i++) {
            peaks[i] = tempPeaks[i];
        }
        return peaks;
    }

    /// @dev Calculate the current root by "bagging the peaks"
    ///
    /// @return root The MMR root
    function _calculateRoot() private view returns (bytes32) {
        if (_remoteCallNonce == 0) {
            return bytes32(0);
        }

        uint256[] memory peakIndices = _getPeakNodeIndices();

        if (peakIndices.length == 0) {
            return bytes32(0);
        }

        return _hashPeaksSequentially(peakIndices);
    }

    /// @dev Hash all peaks sequentially from right to left
    ///
    /// @param peakIndices Array of peak node indices (ordered from rightmost to leftmost)
    ///
    /// @return root The final root hash after hashing all peaks
    function _hashPeaksSequentially(uint256[] memory peakIndices) private view returns (bytes32) {
        // Start with the rightmost peak (first in our reversed list)
        bytes32 currentRoot = _nodes[peakIndices[0]];

        // Sequentially hash with the next peak to the left
        for (uint256 i = 1; i < peakIndices.length; i++) {
            bytes32 nextPeakHash = _nodes[peakIndices[i]];
            currentRoot = _hashInternalNode(nextPeakHash, currentRoot);
        }

        return currentRoot;
    }

    /// @dev Get the indices of all peak nodes in the MMR
    /// Returns peaks ordered from rightmost to leftmost
    function _getPeakNodeIndices() private view returns (uint256[] memory) {
        if (_remoteCallNonce == 0) {
            return new uint256[](0);
        }

        uint256[] memory tempPeakIndices = new uint256[](MAX_PEAKS);
        uint256 peakCount = 0;
        uint256 nodeOffset = 0;
        uint64 remainingLeaves = _remoteCallNonce;

        uint256 maxHeight = _calculateMaxPossibleHeight(_remoteCallNonce);

        // Process each possible height from largest to smallest
        for (uint256 height = maxHeight + 1; height > 0; height--) {
            uint256 currentHeight = height - 1;
            if (_hasCompleteMountainAtHeight(remainingLeaves, currentHeight)) {
                uint256 peakIndex = _calculatePeakIndex(nodeOffset, currentHeight);
                tempPeakIndices[peakCount] = peakIndex;
                peakCount++;

                // Update state for next iteration
                nodeOffset += _calculateMountainSize(currentHeight);
                remainingLeaves -= uint64(1 << currentHeight);
            }
        }

        return _reversePeakIndices(tempPeakIndices, peakCount);
    }

    /// @dev Check if nodes should be merged at the given height based on leaf count
    ///
    /// @param leafCount The number of leaves in the MMR
    /// @param height The height at which to check for merging
    ///
    /// @return shouldMerge True if nodes should be merged at this height
    function _shouldMergeAtHeight(uint64 leafCount, uint256 height) private pure returns (bool) {
        return (leafCount >> height) & 1 == 1;
    }

    /// @dev Calculate the index of the left sibling node
    ///
    /// @param currentNodeIndex The index of the current node
    /// @param height The height of the current level
    ///
    /// @return leftSiblingIndex The index of the left sibling node
    function _calculateLeftSiblingIndex(uint256 currentNodeIndex, uint256 height) private pure returns (uint256) {
        uint256 leftSubtreeSize = (1 << (height + 1)) - 1;
        return currentNodeIndex - leftSubtreeSize;
    }

    /// @dev Calculate the maximum possible height for the given number of leaves
    ///
    /// @param leafCount Number of leaves in the MMR
    ///
    /// @return maxHeight The maximum possible height
    function _calculateMaxPossibleHeight(uint64 leafCount) private pure returns (uint256) {
        if (leafCount == 0) return 0;

        uint256 maxHeight = 0;
        uint64 temp = leafCount;
        while (temp > 0) {
            maxHeight++;
            temp >>= 1;
        }
        return maxHeight > 0 ? maxHeight - 1 : 0;
    }

    /// @dev Check if there's a complete mountain at the given height
    ///
    /// @param leafCount Number of remaining leaves
    /// @param height Height to check
    ///
    /// @return hasCompleteMountain True if there's a complete mountain at this height
    function _hasCompleteMountainAtHeight(uint64 leafCount, uint256 height) private pure returns (bool) {
        return (leafCount >> height) & 1 == 1;
    }

    /// @dev Calculate the peak index for a mountain at the given height
    ///
    /// @param nodeOffset Current offset in the nodes array
    /// @param height Height of the mountain
    ///
    /// @return peakIndex Index of the peak node
    function _calculatePeakIndex(uint256 nodeOffset, uint256 height) private pure returns (uint256) {
        uint256 mountainSize = _calculateMountainSize(height);
        return nodeOffset + mountainSize - 1;
    }

    /// @dev Calculate the number of nodes in a complete mountain of given height
    ///
    /// @param height Height of the mountain
    ///
    /// @return mountainSize Number of nodes in the mountain
    function _calculateMountainSize(uint256 height) private pure returns (uint256) {
        return (1 << (height + 1)) - 1;
    }

    /// @dev Reverse the peak indices array to get the correct order
    ///
    /// @param tempPeakIndices Temporary array containing peak indices
    /// @param peakCount Number of peaks found
    ///
    /// @return peakIndices Reversed array of peak indices
    function _reversePeakIndices(uint256[] memory tempPeakIndices, uint256 peakCount)
        private
        pure
        returns (uint256[] memory)
    {
        uint256[] memory peakIndices = new uint256[](peakCount);
        for (uint256 i = 0; i < peakCount; i++) {
            peakIndices[i] = tempPeakIndices[peakCount - 1 - i];
        }
        return peakIndices;
    }

    /// @dev Internal function to hash two node hashes together
    /// Uses sorted inputs for commutative hashing: H(left, right) == H(right, left)
    function _hashInternalNode(bytes32 left, bytes32 right) private pure returns (bytes32) {
        if (left < right) {
            return keccak256(abi.encodePacked(left, right));
        }
        return keccak256(abi.encodePacked(right, left));
    }

    /// @dev Calculate the population count (number of 1 bits) in a uint64
    /// @param x The number to count bits in
    /// @return count The number of 1 bits
    function _popcount(uint64 x) private pure returns (uint256) {
        uint256 count = 0;
        while (x != 0) {
            count += x & 1;
            x >>= 1;
        }
        return count;
    }
}
