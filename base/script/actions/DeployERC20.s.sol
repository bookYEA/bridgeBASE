// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {Script} from "forge-std/Script.sol";

import {ERC20Mock} from "lib/openzeppelin-contracts/contracts/mocks/ERC20Mock.sol";

contract DeployERC20 is Script {
    function run() public {
        vm.startBroadcast();
        ERC20Mock token = new ERC20Mock();
        token.mint(vm.envAddress("ADMIN"), 100 ether);
        vm.stopBroadcast();
    }
}
