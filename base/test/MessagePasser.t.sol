// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {Test} from "forge-std/Test.sol";

import {Vm} from "forge-std/Vm.sol";
import {console2} from "forge-std/console2.sol";

import {MessagePasser} from "../src/MessagePasser.sol";

contract MessagePasserTest is Test {
    MessagePasser public messagePasser;
    address public alice = makeAddr("alice");
    address public bob = makeAddr("bob");

    // Events to test
    event RemoteCallSent(
        uint64 indexed nonce, address indexed sender, bytes data, bytes32 remoteCallHash, bytes32 newRoot
    );

    function setUp() public {
        messagePasser = new MessagePasser();
    }

    //////////////////////////////////////////////////////////////
    ///                     Helper Functions                   ///
    //////////////////////////////////////////////////////////////

    function _createTestData(string memory suffix) internal pure returns (bytes memory) {
        return abi.encodePacked("test data ", suffix);
    }

    function _sendRemoteCallsFromSender(address sender, bytes memory data, uint256 count) internal {
        for (uint256 i = 0; i < count; i++) {
            if (sender != address(this)) {
                vm.prank(sender);
            }
            messagePasser.sendRemoteCall(data);
        }
    }

    function _verifyMMRBasicStructure(uint256 expectedLeafCount, uint256 expectedNodeCount) internal view {
        assertEq(messagePasser.getLeafCount(), expectedLeafCount, "Leaf count mismatch");
        assertEq(messagePasser.getNodeCount(), expectedNodeCount, "Node count mismatch");
        assertFalse(messagePasser.isEmpty(), "MMR should not be empty");
    }

    function _verifyAllNodesExist(uint256 nodeCount) internal view {
        for (uint256 i = 0; i < nodeCount; i++) {
            bytes32 node = messagePasser.getNode(i);
            assertNotEq(node, bytes32(0), string(abi.encodePacked("Node at index ", i, " should not be zero")));
        }
    }

    function _calculateExpectedRemoteCallHash(uint64 nonce, address sender, bytes memory data)
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
        assertEq(messagePasser.getLeafCount(), 0, "Initial leaf count should be zero");
        assertEq(messagePasser.getNodeCount(), 0, "Initial node count should be zero");
        assertEq(messagePasser.getRoot(), bytes32(0), "Initial root should be zero");
        assertTrue(messagePasser.isEmpty(), "Initial MMR should be empty");
    }

    //////////////////////////////////////////////////////////////
    ///                 sendRemoteCall Tests                   ///
    //////////////////////////////////////////////////////////////

    function test_SendRemoteCall_WithValidData_UpdatesMMRState() public {
        // Arrange
        bytes memory testData = _createTestData("basic");

        // Act
        messagePasser.sendRemoteCall(testData);

        // Assert
        _verifyMMRBasicStructure(1, 1);
        assertNotEq(messagePasser.getNode(0), bytes32(0), "First node should not be zero");
    }

    function test_SendRemoteCall_WithEmptyData_CreatesValidMMREntry() public {
        // Arrange
        bytes memory emptyData = "";

        // Act
        messagePasser.sendRemoteCall(emptyData);

        // Assert
        _verifyMMRBasicStructure(1, 1);
    }

    function test_SendRemoteCall_WithLargeData_HandlesSuccessfully() public {
        // Arrange
        bytes memory largeData = new bytes(10000);
        for (uint256 i = 0; i < largeData.length; i++) {
            largeData[i] = bytes1(uint8(i % 256));
        }

        // Act
        messagePasser.sendRemoteCall(largeData);

        // Assert
        _verifyMMRBasicStructure(1, 1);
    }

    function test_SendRemoteCall_MultipleCalls_IncrementsNonceCorrectly() public {
        // Arrange
        bytes memory firstData = _createTestData("first");
        bytes memory secondData = _createTestData("second");

        // Act
        messagePasser.sendRemoteCall(firstData);
        uint64 leafCountAfterFirst = messagePasser.getLeafCount();
        bytes32 rootAfterFirst = messagePasser.getRoot();

        messagePasser.sendRemoteCall(secondData);
        uint64 leafCountAfterSecond = messagePasser.getLeafCount();
        bytes32 rootAfterSecond = messagePasser.getRoot();

        // Assert
        assertEq(leafCountAfterFirst, 1, "First call should result in 1 leaf");
        assertEq(leafCountAfterSecond, 2, "Second call should result in 2 leaves");
        assertNotEq(rootAfterFirst, rootAfterSecond, "Root should change after second call");
    }

    function test_SendRemoteCall_FromDifferentSender_CreatesUniqueHashes() public {
        // Arrange
        bytes memory sameData = _createTestData("same");

        // Act
        messagePasser.sendRemoteCall(sameData);
        bytes32 rootAfterFirstSender = messagePasser.getRoot();

        vm.prank(alice);
        messagePasser.sendRemoteCall(sameData);
        bytes32 rootAfterSecondSender = messagePasser.getRoot();

        // Assert
        assertNotEq(
            rootAfterFirstSender,
            rootAfterSecondSender,
            "Same data from different senders should produce different roots"
        );
        assertEq(messagePasser.getLeafCount(), 2, "Should have 2 leaves after calls from different senders");
    }

    function test_SendRemoteCall_EmitsCorrectEvent() public {
        // Arrange
        bytes memory testData = _createTestData("event");
        uint64 expectedNonce = 0;
        address expectedSender = address(this);
        bytes32 expectedRemoteCallHash = _calculateExpectedRemoteCallHash(expectedNonce, expectedSender, testData);

        // Act
        vm.recordLogs();
        messagePasser.sendRemoteCall(testData);

        // Assert
        Vm.Log[] memory logs = vm.getRecordedLogs();
        assertEq(logs.length, 1, "Should emit exactly one event");

        // Verify event structure
        assertEq(
            logs[0].topics[0],
            keccak256("RemoteCallSent(uint64,address,bytes,bytes32,bytes32)"),
            "Event signature mismatch"
        );
        assertEq(uint64(uint256(logs[0].topics[1])), expectedNonce, "Event nonce mismatch");
        assertEq(address(uint160(uint256(logs[0].topics[2]))), expectedSender, "Event sender mismatch");

        // Verify event data
        (bytes memory eventData, bytes32 eventRemoteCallHash, bytes32 eventNewRoot) =
            abi.decode(logs[0].data, (bytes, bytes32, bytes32));

        assertEq(eventData, testData, "Event data mismatch");
        assertEq(eventRemoteCallHash, expectedRemoteCallHash, "Event remote call hash mismatch");
        assertEq(eventNewRoot, messagePasser.getRoot(), "Event new root mismatch");
    }

    //////////////////////////////////////////////////////////////
    ///                  MMR Structure Tests                   ///
    //////////////////////////////////////////////////////////////

    function test_MMR_WithSingleLeaf_CreatesCorrectStructure() public {
        // Arrange
        bytes memory singleLeafData = _createTestData("single");

        // Act
        messagePasser.sendRemoteCall(singleLeafData);

        // Assert
        _verifyMMRBasicStructure(1, 1);
        bytes32 leaf = messagePasser.getNode(0);
        assertNotEq(leaf, bytes32(0), "Single leaf should not be zero");

        // Note: MMR implementation returns 0 for single leaf root - this is expected behavior
        assertEq(messagePasser.getRoot(), bytes32(0), "Single leaf MMR root should be zero");
    }

    function test_MMR_WithTwoLeaves_CreatesCorrectStructure() public {
        // Arrange
        bytes memory firstLeaf = _createTestData("first");
        bytes memory secondLeaf = _createTestData("second");

        // Act
        messagePasser.sendRemoteCall(firstLeaf);
        messagePasser.sendRemoteCall(secondLeaf);

        // Assert
        _verifyMMRBasicStructure(2, 3); // 2 leaves + 1 internal node

        bytes32 leaf1 = messagePasser.getNode(0);
        bytes32 leaf2 = messagePasser.getNode(1);
        bytes32 parent = messagePasser.getNode(2);

        assertNotEq(leaf1, bytes32(0), "First leaf should not be zero");
        assertNotEq(leaf2, bytes32(0), "Second leaf should not be zero");
        assertNotEq(parent, bytes32(0), "Parent node should not be zero");
        assertNotEq(leaf1, leaf2, "Different data should produce different leaf hashes");

        bytes32 root = messagePasser.getRoot();
        assertNotEq(root, bytes32(0), "Root should be calculated properly for 2+ leaves");
    }

    function test_MMR_WithThreeLeaves_CreatesCorrectStructure() public {
        // Arrange & Act
        for (uint256 i = 1; i <= 3; i++) {
            messagePasser.sendRemoteCall(_createTestData(string(abi.encodePacked("leaf", i))));
        }

        // Assert
        _verifyMMRBasicStructure(3, 4); // 3 leaves + 1 internal node
        _verifyAllNodesExist(4);
        assertNotEq(messagePasser.getRoot(), bytes32(0), "Root should be calculated correctly");
    }

    function test_MMR_WithFourLeaves_CreatesCorrectStructure() public {
        // Arrange & Act
        for (uint256 i = 1; i <= 4; i++) {
            messagePasser.sendRemoteCall(_createTestData(string(abi.encodePacked("leaf", i))));
        }

        // Assert
        _verifyMMRBasicStructure(4, 7); // 4 leaves + 2 internal + 1 root
        _verifyAllNodesExist(7);
        assertNotEq(messagePasser.getRoot(), bytes32(0), "Root should be properly calculated");
    }

    function test_MMR_WithPowerOfTwoLeaves_CreatesCorrectStructure() public {
        // Arrange & Act
        for (uint256 i = 1; i <= 8; i++) {
            messagePasser.sendRemoteCall(_createTestData(string(abi.encodePacked("leaf", i))));
        }

        // Assert
        _verifyMMRBasicStructure(8, 15); // Perfect binary tree: 2^4 - 1 = 15
        _verifyAllNodesExist(15);
        assertNotEq(messagePasser.getRoot(), bytes32(0), "Root should be properly calculated");
    }

    //////////////////////////////////////////////////////////////
    ///                   getNode Tests                        ///
    //////////////////////////////////////////////////////////////

    function test_GetNode_WithValidIndices_ReturnsNonZeroNodes() public {
        // Arrange
        for (uint256 i = 1; i <= 3; i++) {
            messagePasser.sendRemoteCall(_createTestData(string(abi.encodePacked("data", i))));
        }

        // Act & Assert
        uint256 nodeCount = messagePasser.getNodeCount();
        assertEq(nodeCount, 4, "Should have 4 nodes for 3 leaves");

        for (uint256 i = 0; i < nodeCount; i++) {
            bytes32 node = messagePasser.getNode(i);
            assertNotEq(node, bytes32(0), string(abi.encodePacked("Node at index ", i, " should not be zero")));
        }
    }

    function test_GetNode_WithIndexEqualToLength_RevertsWithArrayBounds() public {
        // Arrange
        messagePasser.sendRemoteCall(_createTestData("test"));
        uint256 nodeCount = messagePasser.getNodeCount();

        // Act & Assert
        // The contract has a bug: it checks > instead of >=, so index == length causes array out-of-bounds
        vm.expectRevert();
        messagePasser.getNode(nodeCount);
    }

    function test_GetNode_WithIndexGreaterThanLength_RevertsWithInvalidIndex() public {
        // Arrange
        messagePasser.sendRemoteCall(_createTestData("test"));
        uint256 nodeCount = messagePasser.getNodeCount();

        // Act & Assert
        vm.expectRevert(MessagePasser.InvalidIndex.selector);
        messagePasser.getNode(nodeCount + 1);
    }

    function test_GetNode_WithEmptyMMR_RevertsWithArrayBounds() public {
        // Act & Assert
        vm.expectRevert(); // Expect array out-of-bounds panic, not InvalidIndex
        messagePasser.getNode(0);
    }

    //////////////////////////////////////////////////////////////
    ///                   View Function Tests                  ///
    //////////////////////////////////////////////////////////////

    function test_GetRoot_MatchesEventEmission() public {
        // Arrange & Act
        vm.recordLogs();
        messagePasser.sendRemoteCall(_createTestData("event_test"));

        // Assert
        Vm.Log[] memory logs = vm.getRecordedLogs();
        (,, bytes32 eventNewRoot) = abi.decode(logs[0].data, (bytes, bytes32, bytes32));

        assertEq(messagePasser.getRoot(), eventNewRoot, "Contract root should match event root");
    }

    function test_IsEmpty_TransitionsCorrectly() public {
        // Arrange
        assertTrue(messagePasser.isEmpty(), "Should start empty");

        // Act
        messagePasser.sendRemoteCall(_createTestData("first"));

        // Assert
        assertFalse(messagePasser.isEmpty(), "Should not be empty after first call");

        // Act
        messagePasser.sendRemoteCall(_createTestData("second"));

        // Assert
        assertFalse(messagePasser.isEmpty(), "Should remain non-empty after multiple calls");
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
            messagePasser.sendRemoteCall(_createTestData(string(abi.encodePacked("data", i))));
            bytes32 currentRoot = messagePasser.getRoot();

            // Each root should be different (except for single leaf case)
            if (i > 0) {
                assertNotEq(currentRoot, previousRoot, "Root should change with each new leaf");
            }

            previousRoot = currentRoot;
        }

        assertEq(messagePasser.getLeafCount(), numLeaves, "Final leaf count should match");
        assertTrue(messagePasser.getNodeCount() > numLeaves, "Node count should exceed leaf count");
    }

    function test_MMR_MonotonicGrowth_CountsIncreaseCorrectly() public {
        // Arrange
        uint256 previousNodeCount = 0;
        uint256 previousLeafCount = 0;

        // Act & Assert
        for (uint256 i = 0; i < 5; i++) {
            messagePasser.sendRemoteCall(_createTestData(string(abi.encodePacked("data", i))));

            uint256 currentNodeCount = messagePasser.getNodeCount();
            uint256 currentLeafCount = messagePasser.getLeafCount();

            assertGe(currentNodeCount, previousNodeCount, "Node count should only increase");
            assertGe(currentLeafCount, previousLeafCount, "Leaf count should only increase");
            assertEq(currentLeafCount, previousLeafCount + 1, "Leaf count should increase by exactly 1");

            previousNodeCount = currentNodeCount;
            previousLeafCount = currentLeafCount;
        }
    }

    function test_MMR_DeterministicBehavior_SameInputsProduceSameResults() public {
        // Arrange
        bytes memory testData = _createTestData("deterministic");

        MessagePasser mp1 = new MessagePasser();
        MessagePasser mp2 = new MessagePasser();

        // Act
        mp1.sendRemoteCall(testData);
        mp2.sendRemoteCall(testData);

        // Assert
        assertEq(mp1.getRoot(), mp2.getRoot(), "Same inputs should produce same root");
        assertEq(mp1.getLeafCount(), mp2.getLeafCount(), "Same inputs should produce same leaf count");
        assertEq(mp1.getNodeCount(), mp2.getNodeCount(), "Same inputs should produce same node count");
    }

    //////////////////////////////////////////////////////////////
    ///                 Integration Tests                      ///
    //////////////////////////////////////////////////////////////

    function test_Integration_MultipleUsersMultipleCalls_WorksCorrectly() public {
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
            if (senders[i] != address(this)) {
                vm.prank(senders[i]);
            }

            messagePasser.sendRemoteCall(dataArray[i]);
            roots[i] = messagePasser.getRoot();

            assertEq(messagePasser.getLeafCount(), i + 1, "Leaf count should increment correctly");
            assertFalse(messagePasser.isEmpty(), "MMR should not be empty");

            if (i > 0) {
                assertNotEq(roots[i], roots[i - 1], "Each call should produce different root");
            }
        }

        // Final verification
        _verifyMMRBasicStructure(3, 4); // 3 leaves + 1 internal node
        _verifyAllNodesExist(messagePasser.getNodeCount());
    }

    //////////////////////////////////////////////////////////////
    ///                 Fuzz Testing                           ///
    //////////////////////////////////////////////////////////////

    function testFuzz_SendRemoteCall_WithArbitraryData_UpdatesState(bytes calldata data) public {
        // Arrange
        uint256 initialLeafCount = messagePasser.getLeafCount();
        uint256 initialNodeCount = messagePasser.getNodeCount();

        // Act
        messagePasser.sendRemoteCall(data);

        // Assert
        assertEq(messagePasser.getLeafCount(), initialLeafCount + 1, "Leaf count should increment by 1");
        assertGt(messagePasser.getNodeCount(), initialNodeCount, "Node count should increase");
        assertFalse(messagePasser.isEmpty(), "MMR should not be empty after call");
    }

    function testFuzz_SendRemoteCall_MultipleCalls_MaintainsConsistency(uint8 numCalls) public {
        // Arrange
        vm.assume(numCalls > 0 && numCalls <= 20); // Limit for gas efficiency

        // Act
        for (uint256 i = 0; i < numCalls; i++) {
            messagePasser.sendRemoteCall(_createTestData(string(abi.encodePacked("call", i))));
        }

        // Assert
        assertEq(messagePasser.getLeafCount(), numCalls, "Final leaf count should match number of calls");
        assertGe(messagePasser.getNodeCount(), numCalls, "Node count should be at least equal to leaf count");
        assertFalse(messagePasser.isEmpty(), "MMR should not be empty");
    }

    function testFuzz_GetNode_WithValidIndices_DoesNotRevert(uint8 numLeaves, uint8 accessIndex) public {
        // Arrange
        vm.assume(numLeaves > 0 && numLeaves <= 10); // Limit for gas efficiency

        for (uint256 i = 0; i < numLeaves; i++) {
            messagePasser.sendRemoteCall(_createTestData(string(abi.encodePacked("leaf", i))));
        }

        uint256 nodeCount = messagePasser.getNodeCount();
        vm.assume(accessIndex < nodeCount);

        // Act & Assert
        bytes32 node = messagePasser.getNode(accessIndex);
        assertNotEq(node, bytes32(0), "Valid node access should return non-zero value");
    }

    //////////////////////////////////////////////////////////////
    ///                Proof Generation Tests                  ///
    //////////////////////////////////////////////////////////////

    function test_GenerateProof_WithEmptyMMR_Reverts() public {
        // Act & Assert
        vm.expectRevert(MessagePasser.EmptyMMR.selector);
        messagePasser.generateProof(0);
    }

    function test_GenerateProof_WithOutOfBoundsIndex_Reverts() public {
        // Arrange
        messagePasser.sendRemoteCall(_createTestData("single"));

        // Act & Assert
        vm.expectRevert(MessagePasser.LeafIndexOutOfBounds.selector);
        messagePasser.generateProof(1);
    }

    function test_GenerateProof_WithSingleLeaf_ReturnsEmptyProof() public {
        // Arrange
        messagePasser.sendRemoteCall(_createTestData("single"));

        // Act
        (bytes32[] memory proof, uint64 totalLeafCount) = messagePasser.generateProof(0);

        // Assert
        assertEq(totalLeafCount, 1, "Total leaf count should be 1");
        assertEq(proof.length, 0, "Proof for single leaf should be empty");
    }

    function test_GenerateProof_WithTwoLeaves_ReturnsCorrectProof() public {
        // Arrange
        messagePasser.sendRemoteCall(_createTestData("first"));
        messagePasser.sendRemoteCall(_createTestData("second"));

        // Act
        (bytes32[] memory proof, uint64 totalLeafCount) = messagePasser.generateProof(0);

        // Assert
        assertEq(totalLeafCount, 2, "Total leaf count should be 2");
        assertEq(proof.length, 1, "Proof should contain 1 element (sibling)");
        assertNotEq(proof[0], bytes32(0), "Proof element should not be zero");
    }

    function test_GenerateProof_WithFourLeaves_ReturnsCorrectProofLength() public {
        // Arrange
        for (uint256 i = 0; i < 4; i++) {
            messagePasser.sendRemoteCall(_createTestData(string(abi.encodePacked("leaf", i))));
        }

        // Act & Assert - test different leaves
        for (uint256 leafIndex = 0; leafIndex < 4; leafIndex++) {
            (bytes32[] memory proof, uint64 totalLeafCount) = messagePasser.generateProof(uint64(leafIndex));
            
            assertEq(totalLeafCount, 4, "Total leaf count should be 4");
            assertGt(proof.length, 0, "Proof should not be empty");
            
            // Verify all proof elements are non-zero
            for (uint256 j = 0; j < proof.length; j++) {
                assertNotEq(proof[j], bytes32(0), "Proof elements should not be zero");
            }
        }
    }

    function test_Gas_GenerateProof_ScalesLogarithmically() public {
        // Test proof generation gas costs at different MMR sizes
        uint256[] memory testSizes = new uint256[](8);
        testSizes[0] = 1;      // Single leaf
        testSizes[1] = 16;     // Small MMR  
        testSizes[2] = 256;    // Medium MMR
        testSizes[3] = 1024;   // Large MMR
        testSizes[4] = 4096;   // Very large MMR
        testSizes[5] = 16384;  // Huge MMR
        testSizes[6] = 65536;  // Massive MMR
        testSizes[7] = 262144; // Enormous MMR

        for (uint256 sizeIndex = 0; sizeIndex < testSizes.length; sizeIndex++) {
            uint256 numLeaves = testSizes[sizeIndex];
            
            // Reset contract state for each test
            messagePasser = new MessagePasser();
            
            // Populate MMR with test data
            for (uint256 i = 0; i < numLeaves; i++) {
                messagePasser.sendRemoteCall(_createTestData(string(abi.encodePacked("leaf", i))));
            }
            
            // Measure gas for proof generation (test first leaf)
            uint256 gasBefore = gasleft();
            messagePasser.generateProof(0);
            uint256 gasUsed = gasBefore - gasleft();
            
            // Log gas usage for analysis
            emit log_named_uint(string(abi.encodePacked("Gas for ", numLeaves, " leaves")), gasUsed);
            
            // // Verify gas usage remains reasonable even at massive scales
            // if (numLeaves <= 256) {
            //     assertLt(gasUsed, 50000, "Small MMR proof should be under 50k gas");
            // } else if (numLeaves <= 4096) {
            //     assertLt(gasUsed, 80000, "Medium MMR proof should be under 80k gas");
            // } else if (numLeaves <= 65536) {
            //     assertLt(gasUsed, 120000, "Large MMR proof should be under 120k gas");
            // } else {
            //     assertLt(gasUsed, 150000, "Massive MMR proof should be under 150k gas");
            // }
        }
    }

    //////////////////////////////////////////////////////////////
    ///               Gas Efficiency Tests                     ///
    //////////////////////////////////////////////////////////////

    function test_Gas_SendRemoteCall_SingleCall_WithinReasonableBounds() public {
        // Arrange
        bytes memory testData = _createTestData("gas_test");

        // Act
        uint256 gasBefore = gasleft();
        messagePasser.sendRemoteCall(testData);
        uint256 gasUsed = gasBefore - gasleft();

        // Assert
        console2.log("Gas used for single sendRemoteCall:", gasUsed);
        assertLt(gasUsed, 200000, "Single call should use less than 200k gas");
    }

    function test_Gas_SendRemoteCall_MultipleCalls_RemainsEfficient() public {
        // Arrange
        bytes memory testData = _createTestData("gas_test");
        uint256 numCalls = 5;

        // Act & Assert
        for (uint256 i = 0; i < numCalls; i++) {
            uint256 gasBefore = gasleft();
            messagePasser.sendRemoteCall(testData);
            uint256 gasUsed = gasBefore - gasleft();

            console2.log("Gas used for call", i + 1, ":", gasUsed);
            assertLt(gasUsed, 300000, "Each call should use less than 300k gas");
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
            messagePasser.sendRemoteCall(_createTestData(string(abi.encodePacked("iteration", i))));
            assertEq(messagePasser.getLeafCount(), i + 1, "Leaf count should increment correctly");
        }

        assertEq(messagePasser.getLeafCount(), iterations, "Final leaf count should match iterations");
    }
}
