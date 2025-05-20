// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {Script} from "forge-std/Script.sol";
import {stdJson} from "forge-std/StdJson.sol";
import {console} from "forge-std/console.sol";
import {ERC20} from "solady/tokens/ERC20.sol";

import {Bridge} from "../../src/Bridge.sol";

contract BridgeTokensToSolanaScript is Script {
    using stdJson for string;

    address public immutable LOCAL_TOKEN = vm.envAddress("LOCAL_TOKEN");
    bytes32 public immutable REMOTE_TOKEN = vm.envBytes32("REMOTE_TOKEN");
    bytes32 public immutable TO = vm.envBytes32("TO");
    uint64 public immutable AMOUNT = uint64(vm.envUint("AMOUNT"));
    bytes public extraData = bytes("Dummy extra data");

    Bridge public bridge;

    function setUp() public {
        Chain memory chain = getChain(block.chainid);
        console.log("Creating token on chain: %s", chain.name);

        string memory rootPath = vm.projectRoot();
        string memory path = string.concat(rootPath, "/deployments/", chain.chainAlias, ".json");
        address bridgeAddress = vm.readFile(path).readAddress(".Bridge");
        bridge = Bridge(bridgeAddress);
    }

    function run() public {
        vm.startBroadcast();
        bridge.bridgeToken(LOCAL_TOKEN, REMOTE_TOKEN, TO, AMOUNT, extraData);
        vm.stopBroadcast();
    }
}
