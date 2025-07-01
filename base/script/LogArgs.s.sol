// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {Script} from "forge-std/Script.sol";
import {console} from "forge-std/console.sol";

contract LogArgsScript is Script {
    function run() public pure {
        address bridge = 0x9766D196CC91D952E146BB71441Fb80d09f9b12E;

        console.log("Twin args:");
        console.logBytes(abi.encode(bridge));

        address owner = 0x0fe884546476dDd290eC46318785046ef68a0BA9;
        address twinImpl = 0x9a1F10711878829BE564Faee5551aF334f7c1553;

        console.log("Upgradeable Beacon args:");
        console.logBytes(abi.encode(owner, twinImpl));

        bytes32 remoteBridge = 0x5547ad75815ba369e7fd8f9a8c37c0c5e1c6f930a68564449d619d21755551b9;
        address trustedRelayer = 0x0e9a877906EBc3b7098DA2404412BF0Ed1A5EFb4;
        address twinBeacon = 0x5F0D9852a09D0286a902d57A9CE184e2A6B0511E;

        console.log("Bridge args:");
        console.logBytes(abi.encode(remoteBridge, trustedRelayer, twinBeacon));

        console.log("CrossChainERC20 args:");
        console.logBytes(abi.encode(bridge));

        address ercImpl = 0xF0DA0C20AbFDCD062d42b1eB9bEC502066bea9D9;

        console.log("ERC20Beacon args:");
        console.logBytes(abi.encode(owner, ercImpl));
    }
}
