// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {Script} from "forge-std/Script.sol";
import {stdJson} from "forge-std/StdJson.sol";
import {console} from "forge-std/console.sol";

import {ERC1967Factory} from "solady/utils/ERC1967Factory.sol";
import {ERC1967FactoryConstants} from "solady/utils/ERC1967FactoryConstants.sol";

import {CrossChainERC20Factory} from "../../src/CrossChainERC20Factory.sol";

contract CreateTokenScript is Script {
    using stdJson for string;

    bytes32 immutable REMOTE_TOKEN = vm.envBytes32("REMOTE_TOKEN");
    string constant NAME = "Test Token";
    string constant SYMBOL = "TT";

    CrossChainERC20Factory crossChainERC20Factory;

    function setUp() public {
        Chain memory chain = getChain(block.chainid);
        console.log("Creating token on chain: %s", chain.name);

        string memory rootPath = vm.projectRoot();
        string memory path = string.concat(rootPath, "/deployments/", chain.chainAlias, ".json");
        address factory = vm.readFile(path).readAddress(".CrossChainERC20Factory");
        crossChainERC20Factory = CrossChainERC20Factory(factory);
    }

    function run() public {
        vm.startBroadcast();
        address token =
            crossChainERC20Factory.deploy({remoteToken: REMOTE_TOKEN, name: NAME, symbol: SYMBOL, decimals: 9});
        console.log("Deployed Token at: %s", token);
        vm.stopBroadcast();
    }
}
