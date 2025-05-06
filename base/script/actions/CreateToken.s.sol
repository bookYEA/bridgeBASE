// SPDX-License-Identifier: MIT
pragma solidity 0.8.15;

import {Script} from "forge-std/Script.sol";
import {stdJson} from "forge-std/StdJson.sol";
import {console} from "forge-std/console.sol";

import {OptimismMintableERC20Factory} from "optimism/packages/contracts-bedrock/src/universal/OptimismMintableERC20Factory.sol";

contract CreateTokenScript is Script {
    using stdJson for string;

    OptimismMintableERC20Factory public immutable FACTORY;
    address public immutable REMOTE_TOKEN;
    string public NAME;
    string public SYMBOL;

    constructor() {
        string memory rootPath = vm.projectRoot();
        string memory path = string.concat(rootPath, "/deployments/", _getChainName(), ".json");
        FACTORY = OptimismMintableERC20Factory(vm.readFile(path).readAddress(".OptimismMintableERC20Factory"));

        REMOTE_TOKEN = vm.envAddress("REMOTE_TOKEN");
        NAME = vm.envString("NAME");
        SYMBOL = vm.envString("SYMBOL");
    }

    function run() public {
        vm.startBroadcast();
        address token = FACTORY.createOptimismMintableERC20(REMOTE_TOKEN, NAME, SYMBOL);
        console.log("Token address: %s", token);
        vm.stopBroadcast();
    }

    function _getChainName() private view returns (string memory) {
        if (block.chainid == 84532) {
            return "baseSepolia";
        } else if (block.chainid == 8453) {
            return "baseMainnet";
        }

        revert("Unsupported chain");
    }
}
