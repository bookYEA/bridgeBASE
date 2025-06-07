// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {Test} from "forge-std/Test.sol";
import {console} from "forge-std/console.sol";

import {Ix, Pubkey, SVMLib} from "../../src/libraries/SVMLib.sol";
import {SVMTokenBridgeLib} from "../../src/libraries/SVMTokenBridgeLib.sol";

contract SVMTokenBridgeLibTest is Test {
    // Pubkey("3R8PyojdmUTwB6FAkzjwRZsfAzucA9E1nK4ydNARvT8b")
    Pubkey constant REMOTE_TOKEN_BRIDGE =
        Pubkey.wrap(0x23e5ad19ec43547dde2b0a2829155a116e4f44d674bd56142d4cc45a64d63108);

    // Pubkey("42424242424242424242424242424242424242424242")
    Pubkey constant RECIPIENT = Pubkey.wrap(0x2cd80de0982d551078e89026dff80f0bfdc03bbf308ca9e6b0bee9feef2d4afb);

    address constant ETH_ADDRESS = 0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE;

    function test_finalizeBridgeTokenIx_localEth() public pure {
        // seeds = [
        //     WRAPPED_TOKEN_SEED,
        //     decimals.to_le_bytes().as_ref(),
        //     metadata.hash().as_ref(),
        // ]
        Pubkey remoteToken = Pubkey.wrap(0x8e4aea0b5d3b4c0a7ecddda65e911fe27e465badbbd1e0ab1b5769f6d2e2a524);
        uint64 remoteAmount = 42_000_000_000;

        Ix memory ix = SVMTokenBridgeLib.finalizeBridgeTokenIx({
            remoteBridge: REMOTE_TOKEN_BRIDGE,
            localToken: ETH_ADDRESS,
            remoteToken: remoteToken,
            to: RECIPIENT,
            remoteAmount: remoteAmount
        });

        Ix[] memory ixs = new Ix[](1);
        ixs[0] = ix;

        console.logBytes(SVMLib.serializeAnchorIxs(ixs));
    }

    function test_finalizeBridgeTokenIx_localERC20() public pure {
        // seeds = [
        //     WRAPPED_TOKEN_SEED,
        //     decimals.to_le_bytes().as_ref(),
        //     metadata.hash().as_ref(),
        // ]
        Pubkey remoteToken = Pubkey.wrap(0xe0b7f7624e2191aee58f622562113118f3e3d3eea8f8b6916726d314e16d6511);
        uint64 remoteAmount = 42_000_000;

        Ix memory ix = SVMTokenBridgeLib.finalizeBridgeTokenIx({
            remoteBridge: REMOTE_TOKEN_BRIDGE,
            localToken: address(0x1234567890123456789012345678901234567890),
            remoteToken: remoteToken,
            to: RECIPIENT,
            remoteAmount: remoteAmount
        });

        Ix[] memory ixs = new Ix[](1);
        ixs[0] = ix;

        console.logBytes(SVMLib.serializeAnchorIxs(ixs));
    }
}
