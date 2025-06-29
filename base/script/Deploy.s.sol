// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {Script} from "forge-std/Script.sol";
import {console} from "forge-std/console.sol";
import {UpgradeableBeacon} from "solady/utils/UpgradeableBeacon.sol";

import {ERC1967Factory} from "solady/utils/ERC1967Factory.sol";

import {Bridge} from "../src/Bridge.sol";
import {CrossChainERC20} from "../src/CrossChainERC20.sol";
import {CrossChainERC20Factory} from "../src/CrossChainERC20Factory.sol";
import {HelperConfig} from "./HelperConfig.s.sol";

contract DeployScript is Script {
    function run() public returns (Bridge, CrossChainERC20Factory, HelperConfig) {
        HelperConfig helperConfig = new HelperConfig();
        HelperConfig.NetworkConfig memory cfg = helperConfig.getConfig();

        Chain memory chain = getChain(block.chainid);
        console.log("Deploying on chain: %s", chain.name);

        vm.startBroadcast();
        address bridge = _deployBridge(cfg);
        address factory = _deployFactory(cfg, bridge);
        vm.stopBroadcast();

        console.log("Deployed Bridge at: %s", bridge);
        console.log("Deployed CrossChainERC20Factory at: %s", factory);

        string memory obj = "root";
        string memory json = vm.serializeAddress(obj, "Bridge", bridge);
        json = vm.serializeAddress(obj, "CrossChainERC20Factory", factory);
        vm.writeJson(json, string.concat("deployments/", chain.chainAlias, ".json"));

        return (Bridge(bridge), CrossChainERC20Factory(factory), helperConfig);
    }

    function _deployBridge(HelperConfig.NetworkConfig memory cfg) private returns (address) {
        Bridge bridgeImpl = new Bridge({remoteBridge: cfg.remoteBridge, trustedRelayer: cfg.trustedRelayer});
        Bridge bridgeProxy = Bridge(
            ERC1967Factory(cfg.erc1967Factory).deployAndCall({
                implementation: address(bridgeImpl),
                admin: cfg.initialOwner,
                data: abi.encodeCall(Bridge.initialize, (cfg.initialOwner))
            })
        );

        return address(bridgeProxy);
    }

    function _deployFactory(HelperConfig.NetworkConfig memory cfg, address bridge) private returns (address) {
        address erc20 = address(new CrossChainERC20(bridge));
        address erc20Beacon =
            address(new UpgradeableBeacon({initialOwner: cfg.initialOwner, initialImplementation: erc20}));

        CrossChainERC20Factory xChainERC20FactoryImpl = new CrossChainERC20Factory(erc20Beacon);
        CrossChainERC20Factory xChainERC20Factory = CrossChainERC20Factory(
            ERC1967Factory(cfg.erc1967Factory).deploy({
                implementation: address(xChainERC20FactoryImpl),
                admin: cfg.initialOwner
            })
        );

        return address(xChainERC20Factory);
    }
}
