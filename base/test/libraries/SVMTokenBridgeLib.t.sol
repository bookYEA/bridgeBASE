// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {Test} from "forge-std/Test.sol";

import {Pubkey} from "../../src/libraries/SVMLib.sol";
import {SVMTokenBridgeLib} from "../../src/libraries/SVMTokenBridgeLib.sol";

contract SVMTokenBridgeLibTest is Test {
    Pubkey private constant PORTAL = Pubkey.wrap(0x352298dbc5fe4de3dafa50254bd3751722e4cb041d452b2361a891b33c940a2f);
    Pubkey private constant REMOTE_BRIDGE =
        Pubkey.wrap(0x23e5ad19ec43547dde2b0a2829155a116e4f44d674bd56142d4cc45a64d63108);

    function test_bridgeTokenIx() public {}
}
