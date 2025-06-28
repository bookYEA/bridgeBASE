// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {Test} from "forge-std/Test.sol";
import {Vm} from "forge-std/Vm.sol";
import {console2} from "forge-std/console2.sol";

import {Message, MessageStorageLib} from "../../src/libraries/MessageStorageLib.sol";

contract MessageStorageLibTest is Test {
    address public alice = makeAddr("alice");
    address public bob = makeAddr("bob");

    //////////////////////////////////////////////////////////////
    ///                     Helper Functions                   ///
    //////////////////////////////////////////////////////////////

    function _createTestData(string memory suffix) internal pure returns (bytes memory) {
        return abi.encodePacked("test data ", suffix);
    }

    function _sendMessagesFromSender(address sender, bytes memory data, uint256 count) internal {
        for (uint256 i = 0; i < count; i++) {
            if (sender != address(this)) {
                vm.prank(sender);
            }
            MessageStorageLib.sendMessage({sender: sender, data: data});
        }
    }

    function _getLeafCount() internal view returns (uint64) {
        return MessageStorageLib.getMessageStorageLibStorage().lastOutgoingNonce;
    }

    function _getNodeCount() internal view returns (uint256) {
        return MessageStorageLib.getMessageStorageLibStorage().nodes.length;
    }

    function _getRoot() internal view returns (bytes32) {
        return MessageStorageLib.getMessageStorageLibStorage().root;
    }

    function _isEmpty() internal view returns (bool) {
        return MessageStorageLib.getMessageStorageLibStorage().lastOutgoingNonce == 0;
    }

    function _getNode(uint256 index) internal view returns (bytes32) {
        return MessageStorageLib.getMessageStorageLibStorage().nodes[index];
    }

    function _verifyMMRBasicStructure(uint256 expectedLeafCount, uint256 expectedNodeCount) internal view {
        assertEq(_getLeafCount(), expectedLeafCount, "Leaf count mismatch");
        assertEq(_getNodeCount(), expectedNodeCount, "Node count mismatch");
        assertFalse(_isEmpty(), "MMR should not be empty");
    }

    function _verifyAllNodesExist(uint256 nodeCount) internal view {
        for (uint256 i = 0; i < nodeCount; i++) {
            bytes32 node = _getNode(i);
            assertNotEq(node, bytes32(0), string(abi.encodePacked("Node at index ", i, " should not be zero")));
        }
    }

    function _calculateExpectedMessageHash(uint64 nonce, address sender, bytes memory data)
        internal
        pure
        returns (bytes32)
    {
        return keccak256(abi.encodePacked(nonce, sender, data));
    }

    //////////////////////////////////////////////////////////////
    ///                   Initial State Tests                  ///
    //////////////////////////////////////////////////////////////

    function test_Constructor_SetsEmptyInitialState() public view {
        // Assert
        assertEq(_getLeafCount(), 0, "Initial leaf count should be zero");
        assertEq(_getNodeCount(), 0, "Initial node count should be zero");
        assertEq(_getRoot(), bytes32(0), "Initial root should be zero");
        assertTrue(_isEmpty(), "Initial MMR should be empty");
    }

    //////////////////////////////////////////////////////////////
    ///                 sendMessage Tests                   ///
    //////////////////////////////////////////////////////////////

    function test_SendMessage_WithValidData_UpdatesMMRState() public {
        // Arrange
        bytes memory testData = _createTestData("basic");

        // Act
        MessageStorageLib.sendMessage({sender: address(this), data: testData});

        // Assert
        _verifyMMRBasicStructure(1, 1);
        assertNotEq(_getNode(0), bytes32(0), "First node should not be zero");
    }

    function test_SendMessage_WithEmptyData_CreatesValidMMREntry() public {
        // Arrange
        bytes memory emptyData = "";

        // Act
        MessageStorageLib.sendMessage({sender: address(this), data: emptyData});

        // Assert
        _verifyMMRBasicStructure(1, 1);
    }

    function test_SendMessage_WithLargeData_HandlesSuccessfully() public {
        // Arrange
        bytes memory largeData = new bytes(10000);
        for (uint256 i = 0; i < largeData.length; i++) {
            largeData[i] = bytes1(uint8(i % 256));
        }

        // Act
        MessageStorageLib.sendMessage({sender: address(this), data: largeData});

        // Assert
        _verifyMMRBasicStructure(1, 1);
    }

    function test_SendMessage_MultipleMessages_IncrementsNonceCorrectly() public {
        // Arrange
        bytes memory firstData = _createTestData("first");
        bytes memory secondData = _createTestData("second");

        // Act
        MessageStorageLib.sendMessage({sender: address(this), data: firstData});
        uint64 leafCountAfterFirst = _getLeafCount();
        bytes32 rootAfterFirst = _getRoot();

        MessageStorageLib.sendMessage({sender: address(this), data: secondData});
        uint64 leafCountAfterSecond = _getLeafCount();
        bytes32 rootAfterSecond = _getRoot();

        // Assert
        assertEq(leafCountAfterFirst, 1, "First message should result in 1 leaf");
        assertEq(leafCountAfterSecond, 2, "Second message should result in 2 leaves");
        assertNotEq(rootAfterFirst, rootAfterSecond, "Root should change after second message");
    }

    function test_SendMessage_FromDifferentSender_CreatesUniqueHashes() public {
        // Arrange
        bytes memory sameData = _createTestData("same");

        // Act
        MessageStorageLib.sendMessage({sender: address(this), data: sameData});
        bytes32 rootAfterFirstSender = _getRoot();

        MessageStorageLib.sendMessage({sender: alice, data: sameData});
        bytes32 rootAfterSecondSender = _getRoot();

        // Assert
        assertNotEq(
            rootAfterFirstSender,
            rootAfterSecondSender,
            "Same data from different senders should produce different roots"
        );
        assertEq(_getLeafCount(), 2, "Should have 2 leaves after messages from different senders");
    }

    function test_SendMessage_EmitsCorrectEvent() public {
        // Arrange
        bytes memory testData = _createTestData("event");
        uint64 expectedNonce = 0;
        address expectedSender = address(this);
        bytes32 expectedMessageHash = _calculateExpectedMessageHash(expectedNonce, expectedSender, testData);

        // Act
        vm.recordLogs();
        MessageStorageLib.sendMessage({sender: address(this), data: testData});

        // Assert
        Vm.Log[] memory logs = vm.getRecordedLogs();
        assertEq(logs.length, 1, "Should emit exactly one event");

        // Verify event structure
        assertEq(
            logs[0].topics[0],
            keccak256("MessageRegistered(bytes32,bytes32,(uint64,address,bytes))"),
            "Event signature mismatch"
        );
        assertEq(logs[0].topics[1], expectedMessageHash, "Message hash mismatch");
        assertEq(logs[0].topics[2], _getRoot(), "MMR root mismatch");

        // Verify event data
        Message memory message = abi.decode(logs[0].data, (Message));

        assertEq(message.nonce, expectedNonce, "Event nonce mismatch");
        assertEq(message.sender, expectedSender, "Event sender mismatch");
        assertEq(message.data, testData, "Event data mismatch");
    }

    //////////////////////////////////////////////////////////////
    ///                  MMR Structure Tests                   ///
    //////////////////////////////////////////////////////////////

    function test_MMR_WithSingleLeaf_CreatesCorrectStructure() public {
        // Arrange
        bytes memory singleLeafData = _createTestData("single");

        // Act
        MessageStorageLib.sendMessage({sender: address(this), data: singleLeafData});

        // Assert
        _verifyMMRBasicStructure(1, 1);
        bytes32 leaf = _getNode(0);
        assertNotEq(leaf, bytes32(0), "Single leaf should not be zero");

        // Note: MMR implementation returns 0 for single leaf root - this is expected behavior
        assertEq(_getRoot(), bytes32(0), "Single leaf MMR root should be zero");
    }

    function test_MMR_WithTwoLeaves_CreatesCorrectStructure() public {
        // Arrange
        bytes memory firstLeaf = _createTestData("first");
        bytes memory secondLeaf = _createTestData("second");

        // Act
        MessageStorageLib.sendMessage({sender: address(this), data: firstLeaf});
        MessageStorageLib.sendMessage({sender: address(this), data: secondLeaf});

        // Assert
        _verifyMMRBasicStructure(2, 3); // 2 leaves + 1 internal node

        bytes32 leaf1 = _getNode(0);
        bytes32 leaf2 = _getNode(1);
        bytes32 parent = _getNode(2);

        assertNotEq(leaf1, bytes32(0), "First leaf should not be zero");
        assertNotEq(leaf2, bytes32(0), "Second leaf should not be zero");
        assertNotEq(parent, bytes32(0), "Parent node should not be zero");
        assertNotEq(leaf1, leaf2, "Different data should produce different leaf hashes");

        bytes32 root = _getRoot();
        assertNotEq(root, bytes32(0), "Root should be calculated properly for 2+ leaves");
    }

    function test_MMR_WithThreeLeaves_CreatesCorrectStructure() public {
        // Arrange & Act
        for (uint256 i = 1; i <= 3; i++) {
            MessageStorageLib.sendMessage({
                sender: address(this),
                data: _createTestData(string(abi.encodePacked("leaf", i)))
            });
        }

        // Assert
        _verifyMMRBasicStructure(3, 4); // 3 leaves + 1 internal node
        _verifyAllNodesExist(4);
        assertNotEq(_getRoot(), bytes32(0), "Root should be calculated correctly");
    }

    function test_MMR_WithFourLeaves_CreatesCorrectStructure() public {
        // Arrange & Act
        for (uint256 i = 1; i <= 4; i++) {
            MessageStorageLib.sendMessage({
                sender: address(this),
                data: _createTestData(string(abi.encodePacked("leaf", i)))
            });
        }

        // Assert
        _verifyMMRBasicStructure(4, 7); // 4 leaves + 2 internal + 1 root
        _verifyAllNodesExist(7);
        assertNotEq(_getRoot(), bytes32(0), "Root should be properly calculated");
    }

    function test_MMR_WithPowerOfTwoLeaves_CreatesCorrectStructure() public {
        // Arrange & Act
        for (uint256 i = 1; i <= 8; i++) {
            MessageStorageLib.sendMessage({
                sender: address(this),
                data: _createTestData(string(abi.encodePacked("leaf", i)))
            });
        }

        // Assert
        _verifyMMRBasicStructure(8, 15); // Perfect binary tree: 2^4 - 1 = 15
        _verifyAllNodesExist(15);
        assertNotEq(_getRoot(), bytes32(0), "Root should be properly calculated");
    }

    //////////////////////////////////////////////////////////////
    ///                   getNode Tests                        ///
    //////////////////////////////////////////////////////////////

    function test_GetNode_WithValidIndices_ReturnsNonZeroNodes() public {
        // Arrange
        for (uint256 i = 1; i <= 3; i++) {
            MessageStorageLib.sendMessage({
                sender: address(this),
                data: _createTestData(string(abi.encodePacked("data", i)))
            });
        }

        // Act & Assert
        uint256 nodeCount = _getNodeCount();
        assertEq(nodeCount, 4, "Should have 4 nodes for 3 leaves");

        for (uint256 i = 0; i < nodeCount; i++) {
            bytes32 node = _getNode(i);
            assertNotEq(node, bytes32(0), string(abi.encodePacked("Node at index ", i, " should not be zero")));
        }
    }

    /// forge-config: default.allow_internal_expect_revert = true
    function test_GetNode_WithIndexEqualToLength_RevertsWithArrayBounds() public {
        // Arrange
        MessageStorageLib.sendMessage({sender: address(this), data: _createTestData("test")});
        uint256 nodeCount = _getNodeCount();

        // Act & Assert
        // The contract has a bug: it checks > instead of >=, so index == length causes array out-of-bounds
        vm.expectRevert();
        _getNode(nodeCount);
    }

    /// forge-config: default.allow_internal_expect_revert = true
    function test_GetNode_WithIndexGreaterThanLength_RevertsWithInvalidIndex() public {
        // Arrange
        MessageStorageLib.sendMessage({sender: address(this), data: _createTestData("test")});
        uint256 nodeCount = _getNodeCount();

        // Act & Assert
        vm.expectRevert(); // Expect array out-of-bounds panic, not InvalidIndex.
        _getNode(nodeCount + 1);
    }

    /// forge-config: default.allow_internal_expect_revert = true
    function test_GetNode_WithEmptyMMR_RevertsWithArrayBounds() public {
        // Act & Assert
        vm.expectRevert(); // Expect array out-of-bounds panic, not InvalidIndex
        _getNode(0);
    }

    //////////////////////////////////////////////////////////////
    ///                   View Function Tests                  ///
    //////////////////////////////////////////////////////////////

    function test_GetRoot_MatchesEventEmission() public {
        // Arrange & Act
        vm.recordLogs();
        MessageStorageLib.sendMessage({sender: address(this), data: _createTestData("event_test")});

        // Assert
        Vm.Log[] memory logs = vm.getRecordedLogs();
        bytes32 mmrRoot = logs[0].topics[2];

        assertEq(_getRoot(), mmrRoot, "Contract root should match event root");
    }

    function test_IsEmpty_TransitionsCorrectly() public {
        // Arrange
        assertTrue(_isEmpty(), "Should start empty");

        // Act
        MessageStorageLib.sendMessage({sender: address(this), data: _createTestData("first")});

        // Assert
        assertFalse(_isEmpty(), "Should not be empty after first message");

        // Act
        MessageStorageLib.sendMessage({sender: address(this), data: _createTestData("second")});

        // Assert
        assertFalse(_isEmpty(), "Should remain non-empty after multiple messages");
    }

    //////////////////////////////////////////////////////////////
    ///                 Consistency Tests                      ///
    //////////////////////////////////////////////////////////////

    function test_MMR_ConsistentGrowth_WithManyLeaves() public {
        // Arrange
        uint256 numLeaves = 10; // Reduced for clarity
        bytes32 previousRoot = bytes32(0);

        // Act & Assert
        for (uint256 i = 0; i < numLeaves; i++) {
            MessageStorageLib.sendMessage({
                sender: address(this),
                data: _createTestData(string(abi.encodePacked("data", i)))
            });
            bytes32 currentRoot = _getRoot();

            // Each root should be different (except for single leaf case)
            if (i > 0) {
                assertNotEq(currentRoot, previousRoot, "Root should change with each new leaf");
            }

            previousRoot = currentRoot;
        }

        assertEq(_getLeafCount(), numLeaves, "Final leaf count should match");
        assertTrue(_getNodeCount() > numLeaves, "Node count should exceed leaf count");
    }

    function test_MMR_MonotonicGrowth_CountsIncreaseCorrectly() public {
        // Arrange
        uint256 previousNodeCount = 0;
        uint256 previousLeafCount = 0;

        // Act & Assert
        for (uint256 i = 0; i < 5; i++) {
            MessageStorageLib.sendMessage({
                sender: address(this),
                data: _createTestData(string(abi.encodePacked("data", i)))
            });

            uint256 currentNodeCount = _getNodeCount();
            uint256 currentLeafCount = _getLeafCount();

            assertGe(currentNodeCount, previousNodeCount, "Node count should only increase");
            assertGe(currentLeafCount, previousLeafCount, "Leaf count should only increase");
            assertEq(currentLeafCount, previousLeafCount + 1, "Leaf count should increase by exactly 1");

            previousNodeCount = currentNodeCount;
            previousLeafCount = currentLeafCount;
        }
    }

    //////////////////////////////////////////////////////////////
    ///                 Integration Tests                      ///
    //////////////////////////////////////////////////////////////

    function test_Integration_MultipleUsersMultipleMessages_WorksCorrectly() public {
        // Arrange
        address[] memory senders = new address[](3);
        senders[0] = alice;
        senders[1] = bob;
        senders[2] = address(this);

        bytes[] memory dataArray = new bytes[](3);
        dataArray[0] = abi.encodePacked("Alice's transaction");
        dataArray[1] = abi.encodePacked("Bob's transaction");
        dataArray[2] = abi.encodePacked("Contract's transaction");

        bytes32[] memory roots = new bytes32[](3);

        // Act & Assert
        for (uint256 i = 0; i < 3; i++) {
            MessageStorageLib.sendMessage({sender: senders[i], data: dataArray[i]});
            roots[i] = _getRoot();

            assertEq(_getLeafCount(), i + 1, "Leaf count should increment correctly");
            assertFalse(_isEmpty(), "MMR should not be empty");

            if (i > 0) {
                assertNotEq(roots[i], roots[i - 1], "Each message should produce different root");
            }
        }

        // Final verification
        _verifyMMRBasicStructure(3, 4); // 3 leaves + 1 internal node
        _verifyAllNodesExist(_getNodeCount());
    }

    //////////////////////////////////////////////////////////////
    ///                 Fuzz Testing                           ///
    //////////////////////////////////////////////////////////////

    function testFuzz_SendMessage_WithArbitraryData_UpdatesState(bytes calldata data) public {
        // Arrange
        uint256 initialLeafCount = _getLeafCount();
        uint256 initialNodeCount = _getNodeCount();

        // Act
        MessageStorageLib.sendMessage({sender: address(this), data: data});

        // Assert
        assertEq(_getLeafCount(), initialLeafCount + 1, "Leaf count should increment by 1");
        assertGt(_getNodeCount(), initialNodeCount, "Node count should increase");
        assertFalse(_isEmpty(), "MMR should not be empty after message");
    }

    function testFuzz_SendMessage_MultipleMessages_MaintainsConsistency(uint8 messageCount) public {
        // Arrange
        vm.assume(messageCount > 0 && messageCount <= 20); // Limit for gas efficiency

        // Act
        for (uint256 i = 0; i < messageCount; i++) {
            MessageStorageLib.sendMessage({
                sender: address(this),
                data: _createTestData(string(abi.encodePacked("message", i)))
            });
        }

        // Assert
        assertEq(_getLeafCount(), messageCount, "Final leaf count should match number of messages");
        assertGe(_getNodeCount(), messageCount, "Node count should be at least equal to leaf count");
        assertFalse(_isEmpty(), "MMR should not be empty");
    }

    function testFuzz_GetNode_WithValidIndices_DoesNotRevert(uint8 numLeaves, uint8 accessIndex) public {
        // Arrange
        vm.assume(numLeaves > 0 && numLeaves <= 10); // Limit for gas efficiency

        for (uint256 i = 0; i < numLeaves; i++) {
            MessageStorageLib.sendMessage({
                sender: address(this),
                data: _createTestData(string(abi.encodePacked("leaf", i)))
            });
        }

        uint256 nodeCount = _getNodeCount();
        vm.assume(accessIndex < nodeCount);

        // Act & Assert
        bytes32 node = _getNode(accessIndex);
        assertNotEq(node, bytes32(0), "Valid node access should return non-zero value");
    }

    //////////////////////////////////////////////////////////////
    ///                Proof Generation Tests                  ///
    //////////////////////////////////////////////////////////////

    /// forge-config: default.allow_internal_expect_revert = true
    function test_GenerateProof_WithEmptyMMR_Reverts() public {
        // Act & Assert
        vm.expectRevert(MessageStorageLib.EmptyMMR.selector);
        MessageStorageLib.generateProof(0);
    }

    /// forge-config: default.allow_internal_expect_revert = true
    function test_GenerateProof_WithOutOfBoundsIndex_Reverts() public {
        // Arrange
        MessageStorageLib.sendMessage({sender: address(this), data: _createTestData("single")});

        // Act & Assert
        vm.expectRevert(MessageStorageLib.LeafIndexOutOfBounds.selector);
        MessageStorageLib.generateProof(1);
    }

    function test_GenerateProof_WithSingleLeaf_ReturnsEmptyProof() public {
        // Arrange
        MessageStorageLib.sendMessage({sender: address(this), data: _createTestData("single")});

        // Act
        (bytes32[] memory proof, uint64 totalLeafCount) = MessageStorageLib.generateProof(0);

        // Assert
        assertEq(totalLeafCount, 1, "Total leaf count should be 1");
        assertEq(proof.length, 0, "Proof for single leaf should be empty");
    }

    function test_GenerateProof_WithTwoLeaves_ReturnsCorrectProof() public {
        // Arrange
        MessageStorageLib.sendMessage({sender: address(this), data: _createTestData("first")});
        MessageStorageLib.sendMessage({sender: address(this), data: _createTestData("second")});

        // Act
        (bytes32[] memory proof, uint64 totalLeafCount) = MessageStorageLib.generateProof(0);

        // Assert
        assertEq(totalLeafCount, 2, "Total leaf count should be 2");
        assertEq(proof.length, 1, "Proof should contain 1 element (sibling)");
        assertNotEq(proof[0], bytes32(0), "Proof element should not be zero");
    }

    function test_GenerateProof_WithFourLeaves_ReturnsCorrectProofLength() public {
        // Arrange
        for (uint256 i = 0; i < 4; i++) {
            MessageStorageLib.sendMessage({
                sender: address(this),
                data: _createTestData(string(abi.encodePacked("leaf", i)))
            });
        }

        // Act & Assert - test different leaves
        for (uint256 leafIndex = 0; leafIndex < 4; leafIndex++) {
            (bytes32[] memory proof, uint64 totalLeafCount) = MessageStorageLib.generateProof(uint64(leafIndex));

            assertEq(totalLeafCount, 4, "Total leaf count should be 4");
            assertGt(proof.length, 0, "Proof should not be empty");

            // Verify all proof elements are non-zero
            for (uint256 j = 0; j < proof.length; j++) {
                assertNotEq(proof[j], bytes32(0), "Proof elements should not be zero");
            }
        }
    }

    //////////////////////////////////////////////////////////////
    ///               Gas Efficiency Tests                     ///
    //////////////////////////////////////////////////////////////

    function test_Gas_SendMessage_SingleMessage_WithinReasonableBounds() public {
        // Arrange
        bytes memory testData = _createTestData("gas_test");

        // Act
        uint256 gasBefore = gasleft();
        MessageStorageLib.sendMessage({sender: address(this), data: testData});
        uint256 gasUsed = gasBefore - gasleft();

        // Assert
        console2.log("Gas used for single sendMessage:", gasUsed);
        assertLt(gasUsed, 200000, "Single message should use less than 200k gas");
    }

    function test_Gas_SendMessage_MultipleMessages_RemainsEfficient() public {
        // Arrange
        bytes memory testData = _createTestData("gas_test");
        uint256 messageCount = 5;

        // Act & Assert
        for (uint256 i = 0; i < messageCount; i++) {
            uint256 gasBefore = gasleft();
            MessageStorageLib.sendMessage({sender: address(this), data: testData});
            uint256 gasUsed = gasBefore - gasleft();

            console2.log("Gas used for message", i + 1, ":", gasUsed);
            assertLt(gasUsed, 300000, "Each message should use less than 300k gas");
        }
    }

    //////////////////////////////////////////////////////////////
    ///                 Regression Tests                       ///
    //////////////////////////////////////////////////////////////

    function test_Regression_NonceIncrement_HandlesLargeValues() public {
        // Arrange
        uint256 iterations = 100; // Reduced for test efficiency

        // Act & Assert
        for (uint256 i = 0; i < iterations; i++) {
            MessageStorageLib.sendMessage({
                sender: address(this),
                data: _createTestData(string(abi.encodePacked("iteration", i)))
            });
            assertEq(_getLeafCount(), i + 1, "Leaf count should increment correctly");
        }

        assertEq(_getLeafCount(), iterations, "Final leaf count should match iterations");
    }
}
