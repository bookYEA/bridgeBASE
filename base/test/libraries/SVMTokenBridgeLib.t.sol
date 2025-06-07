// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {Test} from "forge-std/Test.sol";
import {console} from "forge-std/console.sol";

import {Ix, Pubkey, SVMLib} from "../../src/libraries/SVMLib.sol";
import {SVMTokenBridgeLib} from "../../src/libraries/SVMTokenBridgeLib.sol";

contract SVMTokenBridgeLibTest is Test {
    //////////////////////////////////////////////////////////////
    ///                       Constants                        ///
    //////////////////////////////////////////////////////////////

    // Pubkey("3R8PyojdmUTwB6FAkzjwRZsfAzucA9E1nK4ydNARvT8b")
    Pubkey constant REMOTE_TOKEN_BRIDGE =
        Pubkey.wrap(0x23e5ad19ec43547dde2b0a2829155a116e4f44d674bd56142d4cc45a64d63108);

    // Pubkey("42424242424242424242424242424242424242424242")
    Pubkey constant RECIPIENT = Pubkey.wrap(0x2cd80de0982d551078e89026dff80f0bfdc03bbf308ca9e6b0bee9feef2d4afb);

    address constant ETH_ADDRESS = 0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE;
    address constant ERC20_ADDRESS = 0x1234567890123456789012345678901234567890;

    //////////////////////////////////////////////////////////////
    ///                    Test Structures                     ///
    //////////////////////////////////////////////////////////////

    struct FinalizeBridgeTokenTestCase {
        Pubkey remoteBridge;
        address localToken;
        Pubkey remoteToken;
        Pubkey to;
        uint64 remoteAmount;
        bytes expected;
        string description;
    }

    struct FinalizeBridgeSolTestCase {
        Pubkey remoteBridge;
        address localToken;
        Pubkey to;
        uint64 remoteAmount;
        bytes expected;
        string description;
    }

    struct FinalizeBridgeSplTestCase {
        Pubkey remoteBridge;
        address localToken;
        Pubkey remoteToken;
        Pubkey to;
        uint64 remoteAmount;
        bytes expected;
        string description;
    }

    //////////////////////////////////////////////////////////////
    ///               finalizeBridgeTokenIx Tests              ///
    //////////////////////////////////////////////////////////////

    function test_finalizeBridgeTokenIx() public pure {
        FinalizeBridgeTokenTestCase[] memory testCases = new FinalizeBridgeTokenTestCase[](4);

        // Test case 0: Basic ETH bridging
        testCases[0] = FinalizeBridgeTokenTestCase({
            remoteBridge: REMOTE_TOKEN_BRIDGE,
            localToken: ETH_ADDRESS,
            remoteToken: Pubkey.wrap(0xd051a0a4c55e1009105540d3c4dd5e256f09f06c05cf99fca539be0d2afa147c),
            to: RECIPIENT,
            remoteAmount: 42_424_242_424,
            expected: hex"23e5ad19ec43547dde2b0a2829155a116e4f44d674bd56142d4cc45a64d631080300000000d051a0a4c55e1009105540d3c4dd5e256f09f06c05cf99fca539be0d2afa147c0100002cd80de0982d551078e89026dff80f0bfdc03bbf308ca9e6b0bee9feef2d4afb01000006ddf6e1ee758fde18425dbce46ccddab61afc4d83b90d27febdf928d8a18bfc000024000000d7ddaf6a29f4eb02eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeef890aee009000000",
            description: "test case 0"
        });

        // Test case 1: ERC20 token bridging
        testCases[1] = FinalizeBridgeTokenTestCase({
            remoteBridge: REMOTE_TOKEN_BRIDGE,
            localToken: ERC20_ADDRESS,
            remoteToken: Pubkey.wrap(0xde14189fc9f9c9ee6f2da8eceb34e881bba993b090796f3187732750b4da473b),
            to: RECIPIENT,
            remoteAmount: 42_000_000,
            expected: hex"23e5ad19ec43547dde2b0a2829155a116e4f44d674bd56142d4cc45a64d631080300000000de14189fc9f9c9ee6f2da8eceb34e881bba993b090796f3187732750b4da473b0100002cd80de0982d551078e89026dff80f0bfdc03bbf308ca9e6b0bee9feef2d4afb01000006ddf6e1ee758fde18425dbce46ccddab61afc4d83b90d27febdf928d8a18bfc000024000000d7ddaf6a29f4eb02123456789012345678901234567890123456789080de800200000000",
            description: "test case 1"
        });

        // Test case 2: Zero amount
        testCases[2] = FinalizeBridgeTokenTestCase({
            remoteBridge: REMOTE_TOKEN_BRIDGE,
            localToken: ERC20_ADDRESS,
            remoteToken: Pubkey.wrap(0xde14189fc9f9c9ee6f2da8eceb34e881bba993b090796f3187732750b4da473b),
            to: RECIPIENT,
            remoteAmount: 0,
            expected: hex"23e5ad19ec43547dde2b0a2829155a116e4f44d674bd56142d4cc45a64d631080300000000de14189fc9f9c9ee6f2da8eceb34e881bba993b090796f3187732750b4da473b0100002cd80de0982d551078e89026dff80f0bfdc03bbf308ca9e6b0bee9feef2d4afb01000006ddf6e1ee758fde18425dbce46ccddab61afc4d83b90d27febdf928d8a18bfc000024000000d7ddaf6a29f4eb0212345678901234567890123456789012345678900000000000000000",
            description: "test case 2"
        });

        // Test case 3: Maximum amount
        testCases[3] = FinalizeBridgeTokenTestCase({
            remoteBridge: REMOTE_TOKEN_BRIDGE,
            localToken: ERC20_ADDRESS,
            remoteToken: Pubkey.wrap(0xde14189fc9f9c9ee6f2da8eceb34e881bba993b090796f3187732750b4da473b),
            to: RECIPIENT,
            remoteAmount: type(uint64).max,
            expected: hex"23e5ad19ec43547dde2b0a2829155a116e4f44d674bd56142d4cc45a64d631080300000000de14189fc9f9c9ee6f2da8eceb34e881bba993b090796f3187732750b4da473b0100002cd80de0982d551078e89026dff80f0bfdc03bbf308ca9e6b0bee9feef2d4afb01000006ddf6e1ee758fde18425dbce46ccddab61afc4d83b90d27febdf928d8a18bfc000024000000d7ddaf6a29f4eb021234567890123456789012345678901234567890ffffffffffffffff",
            description: "test case 3"
        });

        // Build, serialize, and verify each test case
        for (uint256 i = 0; i < testCases.length; i++) {
            // Build and serialize the instruction
            bytes memory serializedIx = SVMLib.serializeAnchorIx(
                SVMTokenBridgeLib.finalizeBridgeTokenIx({
                    remoteBridge: testCases[i].remoteBridge,
                    localToken: testCases[i].localToken,
                    remoteToken: testCases[i].remoteToken,
                    to: testCases[i].to,
                    remoteAmount: testCases[i].remoteAmount
                })
            );

            // Verify serialization matches expected
            assertEq(
                serializedIx,
                testCases[i].expected,
                string(abi.encodePacked("finalizeBridgeTokenIx serialization failed for ", testCases[i].description))
            );
        }
    }

    //////////////////////////////////////////////////////////////
    ///                finalizeBridgeSolIx Tests               ///
    //////////////////////////////////////////////////////////////

    function test_finalizeBridgeSolIx() public pure {
        FinalizeBridgeSolTestCase[] memory testCases = new FinalizeBridgeSolTestCase[](4);

        // Test case 0: Basic ETH SOL bridging
        testCases[0] = FinalizeBridgeSolTestCase({
            remoteBridge: REMOTE_TOKEN_BRIDGE,
            localToken: ETH_ADDRESS,
            to: RECIPIENT,
            remoteAmount: 42_424_242_424,
            expected: hex"23e5ad19ec43547dde2b0a2829155a116e4f44d674bd56142d4cc45a64d6310803000000010200000009000000736f6c5f7661756c7414000000eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee23e5ad19ec43547dde2b0a2829155a116e4f44d674bd56142d4cc45a64d631080100002cd80de0982d551078e89026dff80f0bfdc03bbf308ca9e6b0bee9feef2d4afb0100000000000000000000000000000000000000000000000000000000000000000000000024000000c5b9b4cce432e255eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeef890aee009000000",
            description: "test case 0"
        });

        // Test case 1: ERC20 SOL bridging
        testCases[1] = FinalizeBridgeSolTestCase({
            remoteBridge: REMOTE_TOKEN_BRIDGE,
            localToken: ERC20_ADDRESS,
            to: RECIPIENT,
            remoteAmount: 42_000_000,
            expected: hex"23e5ad19ec43547dde2b0a2829155a116e4f44d674bd56142d4cc45a64d6310803000000010200000009000000736f6c5f7661756c7414000000123456789012345678901234567890123456789023e5ad19ec43547dde2b0a2829155a116e4f44d674bd56142d4cc45a64d631080100002cd80de0982d551078e89026dff80f0bfdc03bbf308ca9e6b0bee9feef2d4afb0100000000000000000000000000000000000000000000000000000000000000000000000024000000c5b9b4cce432e255123456789012345678901234567890123456789080de800200000000",
            description: "test case 1"
        });

        // Test case 2: Zero amount
        testCases[2] = FinalizeBridgeSolTestCase({
            remoteBridge: REMOTE_TOKEN_BRIDGE,
            localToken: ERC20_ADDRESS,
            to: RECIPIENT,
            remoteAmount: 0,
            expected: hex"23e5ad19ec43547dde2b0a2829155a116e4f44d674bd56142d4cc45a64d6310803000000010200000009000000736f6c5f7661756c7414000000123456789012345678901234567890123456789023e5ad19ec43547dde2b0a2829155a116e4f44d674bd56142d4cc45a64d631080100002cd80de0982d551078e89026dff80f0bfdc03bbf308ca9e6b0bee9feef2d4afb0100000000000000000000000000000000000000000000000000000000000000000000000024000000c5b9b4cce432e25512345678901234567890123456789012345678900000000000000000",
            description: "test case 2"
        });

        // Test case 3: Maximum amount
        testCases[3] = FinalizeBridgeSolTestCase({
            remoteBridge: REMOTE_TOKEN_BRIDGE,
            localToken: ERC20_ADDRESS,
            to: RECIPIENT,
            remoteAmount: type(uint64).max,
            expected: hex"23e5ad19ec43547dde2b0a2829155a116e4f44d674bd56142d4cc45a64d6310803000000010200000009000000736f6c5f7661756c7414000000123456789012345678901234567890123456789023e5ad19ec43547dde2b0a2829155a116e4f44d674bd56142d4cc45a64d631080100002cd80de0982d551078e89026dff80f0bfdc03bbf308ca9e6b0bee9feef2d4afb0100000000000000000000000000000000000000000000000000000000000000000000000024000000c5b9b4cce432e2551234567890123456789012345678901234567890ffffffffffffffff",
            description: "test case 3"
        });

        // Build, serialize, and verify each test case
        for (uint256 i = 0; i < testCases.length; i++) {
            // Build and serialize the instruction
            bytes memory serializedIx = SVMLib.serializeAnchorIx(
                SVMTokenBridgeLib.finalizeBridgeSolIx({
                    remoteBridge: testCases[i].remoteBridge,
                    localToken: testCases[i].localToken,
                    to: testCases[i].to,
                    remoteAmount: testCases[i].remoteAmount
                })
            );

            // Verify serialization matches expected
            assertEq(
                serializedIx,
                testCases[i].expected,
                string(abi.encodePacked("finalizeBridgeSolIx serialization failed for ", testCases[i].description))
            );
        }
    }

    //////////////////////////////////////////////////////////////
    ///                finalizeBridgeSplIx Tests               ///
    //////////////////////////////////////////////////////////////

    function test_finalizeBridgeSplIx() public pure {
        FinalizeBridgeSplTestCase[] memory testCases = new FinalizeBridgeSplTestCase[](4);

        // Test case 0: Basic ETH SPL bridging
        testCases[0] = FinalizeBridgeSplTestCase({
            remoteBridge: REMOTE_TOKEN_BRIDGE,
            localToken: ETH_ADDRESS,
            remoteToken: Pubkey.wrap(0xb35864d1d0a8a5c6cf2e2931e26868127032d11de4a4f840c3b6d2fe1b0a51e3),
            to: RECIPIENT,
            remoteAmount: 42_424_242_424,
            expected: hex"23e5ad19ec43547dde2b0a2829155a116e4f44d674bd56142d4cc45a64d631080400000000b35864d1d0a8a5c6cf2e2931e26868127032d11de4a4f840c3b6d2fe1b0a51e3010001030000000b000000746f6b656e5f7661756c7420000000b35864d1d0a8a5c6cf2e2931e26868127032d11de4a4f840c3b6d2fe1b0a51e314000000eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee23e5ad19ec43547dde2b0a2829155a116e4f44d674bd56142d4cc45a64d631080100002cd80de0982d551078e89026dff80f0bfdc03bbf308ca9e6b0bee9feef2d4afb01000006ddf6e1ee758fde18425dbce46ccddab61afc4d83b90d27febdf928d8a18bfc000024000000833e369d8bfdfb6ceeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeef890aee009000000",
            description: "test case 0"
        });

        // Test case 1: ERC20 SPL bridging
        testCases[1] = FinalizeBridgeSplTestCase({
            remoteBridge: REMOTE_TOKEN_BRIDGE,
            localToken: ERC20_ADDRESS,
            remoteToken: Pubkey.wrap(0x13bc4e23fe7260386de7aba59312b70b421f4375037b77fba865b0cb7fefbf69),
            to: RECIPIENT,
            remoteAmount: 42_000_000,
            expected: hex"23e5ad19ec43547dde2b0a2829155a116e4f44d674bd56142d4cc45a64d63108040000000013bc4e23fe7260386de7aba59312b70b421f4375037b77fba865b0cb7fefbf69010001030000000b000000746f6b656e5f7661756c742000000013bc4e23fe7260386de7aba59312b70b421f4375037b77fba865b0cb7fefbf6914000000123456789012345678901234567890123456789023e5ad19ec43547dde2b0a2829155a116e4f44d674bd56142d4cc45a64d631080100002cd80de0982d551078e89026dff80f0bfdc03bbf308ca9e6b0bee9feef2d4afb01000006ddf6e1ee758fde18425dbce46ccddab61afc4d83b90d27febdf928d8a18bfc000024000000833e369d8bfdfb6c123456789012345678901234567890123456789080de800200000000",
            description: "test case 1"
        });

        // Test case 2: Zero amount
        testCases[2] = FinalizeBridgeSplTestCase({
            remoteBridge: REMOTE_TOKEN_BRIDGE,
            localToken: ERC20_ADDRESS,
            remoteToken: Pubkey.wrap(0x13bc4e23fe7260386de7aba59312b70b421f4375037b77fba865b0cb7fefbf69),
            to: RECIPIENT,
            remoteAmount: 0,
            expected: hex"23e5ad19ec43547dde2b0a2829155a116e4f44d674bd56142d4cc45a64d63108040000000013bc4e23fe7260386de7aba59312b70b421f4375037b77fba865b0cb7fefbf69010001030000000b000000746f6b656e5f7661756c742000000013bc4e23fe7260386de7aba59312b70b421f4375037b77fba865b0cb7fefbf6914000000123456789012345678901234567890123456789023e5ad19ec43547dde2b0a2829155a116e4f44d674bd56142d4cc45a64d631080100002cd80de0982d551078e89026dff80f0bfdc03bbf308ca9e6b0bee9feef2d4afb01000006ddf6e1ee758fde18425dbce46ccddab61afc4d83b90d27febdf928d8a18bfc000024000000833e369d8bfdfb6c12345678901234567890123456789012345678900000000000000000",
            description: "test case 2"
        });

        // Test case 3: Maximum amount
        testCases[3] = FinalizeBridgeSplTestCase({
            remoteBridge: REMOTE_TOKEN_BRIDGE,
            localToken: ERC20_ADDRESS,
            remoteToken: Pubkey.wrap(0x13bc4e23fe7260386de7aba59312b70b421f4375037b77fba865b0cb7fefbf69),
            to: RECIPIENT,
            remoteAmount: type(uint64).max,
            expected: hex"23e5ad19ec43547dde2b0a2829155a116e4f44d674bd56142d4cc45a64d63108040000000013bc4e23fe7260386de7aba59312b70b421f4375037b77fba865b0cb7fefbf69010001030000000b000000746f6b656e5f7661756c742000000013bc4e23fe7260386de7aba59312b70b421f4375037b77fba865b0cb7fefbf6914000000123456789012345678901234567890123456789023e5ad19ec43547dde2b0a2829155a116e4f44d674bd56142d4cc45a64d631080100002cd80de0982d551078e89026dff80f0bfdc03bbf308ca9e6b0bee9feef2d4afb01000006ddf6e1ee758fde18425dbce46ccddab61afc4d83b90d27febdf928d8a18bfc000024000000833e369d8bfdfb6c1234567890123456789012345678901234567890ffffffffffffffff",
            description: "test case 3"
        });

        // Build, serialize, and verify each test case
        for (uint256 i = 0; i < testCases.length; i++) {
            // Build and serialize the instruction
            bytes memory serializedIx = SVMLib.serializeAnchorIx(
                SVMTokenBridgeLib.finalizeBridgeSplIx({
                    remoteBridge: testCases[i].remoteBridge,
                    localToken: testCases[i].localToken,
                    remoteToken: testCases[i].remoteToken,
                    to: testCases[i].to,
                    remoteAmount: testCases[i].remoteAmount
                })
            );

            // Verify serialization matches expected
            assertEq(
                serializedIx,
                testCases[i].expected,
                string(abi.encodePacked("finalizeBridgeSplIx serialization failed for ", testCases[i].description))
            );
        }
    }
}
