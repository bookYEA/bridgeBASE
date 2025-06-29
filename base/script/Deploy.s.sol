// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {Script} from "forge-std/Script.sol";
import {console} from "forge-std/console.sol";
import {UpgradeableBeacon} from "solady/utils/UpgradeableBeacon.sol";

import {ERC1967Factory} from "solady/utils/ERC1967Factory.sol";
import {ERC1967FactoryConstants} from "solady/utils/ERC1967FactoryConstants.sol";

import {Bridge} from "../src/Bridge.sol";
import {CrossChainERC20} from "../src/CrossChainERC20.sol";
import {CrossChainERC20Factory} from "../src/CrossChainERC20Factory.sol";

import {Pubkey} from "../src/libraries/SVMLib.sol";

contract DeployScript is Script {
    address public constant PROXY_ADMIN = 0x0fe884546476dDd290eC46318785046ef68a0BA9;

    // EF3xsxZGWWJX9T7vCPb7hEgyJQKEj1mgSNLMNvF8a7cj
    Pubkey public constant REMOTE_BRIDGE =
        Pubkey.wrap(0xc4c16980efe2a570c1a7599fd2ebb40ca7f85daf897482b9c85d4b8933a61608);
    address public constant ORACLE = 0x0e9a877906EBc3b7098DA2404412BF0Ed1A5EFb4;

    function setUp() public {
        vm.label(PROXY_ADMIN, "PROXY_ADMIN");
        vm.label(ERC1967FactoryConstants.ADDRESS, "ERC1967_FACTORY");
    }

    function run() public {
        Chain memory chain = getChain(block.chainid);
        console.log("Deploying on chain: %s", chain.name);

        vm.startBroadcast();
        address bridge = _deployBridge();
        address factory = _deployFactory(bridge);
        vm.stopBroadcast();

        console.log("Deployed Bridge at: %s", bridge);
        console.log("Deployed CrossChainERC20Factory at: %s", factory);

        string memory obj = "root";
        string memory json = vm.serializeAddress(obj, "Bridge", bridge);
        json = vm.serializeAddress(obj, "CrossChainERC20Factory", factory);
        vm.writeJson(json, string.concat("deployments/", chain.chainAlias, ".json"));
    }

    function _deployBridge() private returns (address) {
        Bridge bridgeImpl = new Bridge({remoteBridge: REMOTE_BRIDGE, trustedRelayer: ORACLE});
        Bridge bridgeProxy = Bridge(
            ERC1967Factory(ERC1967FactoryConstants.ADDRESS).deployAndCall({
                implementation: address(bridgeImpl),
                admin: PROXY_ADMIN,
                data: abi.encodeCall(Bridge.initialize, (PROXY_ADMIN))
            })
        );

        return address(bridgeProxy);
    }

    function _deployFactory(address bridge) private returns (address) {
        address erc20 = address(new CrossChainERC20(bridge));
        address erc20Beacon = address(new UpgradeableBeacon({initialOwner: PROXY_ADMIN, initialImplementation: erc20}));

        CrossChainERC20Factory xChainERC20FactoryImpl = new CrossChainERC20Factory(erc20Beacon);
        CrossChainERC20Factory xChainERC20Factory = CrossChainERC20Factory(
            ERC1967Factory(ERC1967FactoryConstants.ADDRESS).deploy({
                implementation: address(xChainERC20FactoryImpl),
                admin: PROXY_ADMIN
            })
        );

        return address(xChainERC20Factory);
    }
}
