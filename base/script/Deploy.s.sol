// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {Script} from "forge-std/Script.sol";
import {console} from "forge-std/console.sol";

import {ERC1967Factory} from "solady/utils/ERC1967Factory.sol";
import {ERC1967FactoryConstants} from "solady/utils/ERC1967FactoryConstants.sol";

import {Bridge} from "../src/Bridge.sol";
import {CrossChainERC20Factory} from "../src/CrossChainERC20Factory.sol";
import {CrossChainMessenger} from "../src/CrossChainMessenger.sol";

contract DeployScript is Script {
    address public constant PROXY_ADMIN = 0x0fe884546476dDd290eC46318785046ef68a0BA9;

    bytes32 public constant REMOTE_MESSENGER = 0x0000000000000000000000000e9a877906EBc3b7098DA2404412BF0Ed1A5EFb4;
    bytes32 public constant OTHER_BRIDGE = 0x7a25452c36304317d6fe970091c383b0d45e9b0b06485d2561156f025c6936af;

    function setUp() public {
        vm.label(PROXY_ADMIN, "PROXY_ADMIN");
        vm.label(ERC1967FactoryConstants.ADDRESS, "ERC1967_FACTORY");
    }

    function run() public {
        Chain memory chain = getChain(block.chainid);
        console.log("Deploying on chain: %s", chain.name);

        vm.startBroadcast();
        address messenger = _deployMessenger();
        address bridge = _deployBridge(messenger);
        address factory = _deployFactory(bridge);
        vm.stopBroadcast();

        console.log("Deployed CrossChainMessenger at: %s", messenger);
        console.log("Deployed Bridge at: %s", bridge);
        console.log("Deployed CrossChainERC20Factory at: %s", factory);

        string memory out = "{";
        out = _record(out, "CrossChainMessenger", messenger, false);
        out = _record(out, "Bridge", bridge, false);
        out = _record(out, "CrossChainERC20Factory", factory, true);
        out = string.concat(out, "}");

        vm.createDir("deployments", true);
        vm.writeFile(string.concat("deployments/", chain.chainAlias, ".json"), out);
    }

    function _deployMessenger() private returns (address) {
        CrossChainMessenger messengerImpl = new CrossChainMessenger();
        CrossChainMessenger messengerProxy = CrossChainMessenger(
            ERC1967Factory(ERC1967FactoryConstants.ADDRESS).deployAndCall({
                implementation: address(messengerImpl),
                admin: PROXY_ADMIN,
                data: abi.encodeCall(CrossChainMessenger.initialize, (REMOTE_MESSENGER))
            })
        );

        return address(messengerProxy);
    }

    function _deployBridge(address messenger) private returns (address) {
        Bridge bridgeImpl = new Bridge();
        Bridge bridgeProxy = Bridge(
            ERC1967Factory(ERC1967FactoryConstants.ADDRESS).deployAndCall({
                implementation: address(bridgeImpl),
                admin: PROXY_ADMIN,
                data: abi.encodeCall(Bridge.initialize, (messenger, OTHER_BRIDGE))
            })
        );

        return address(bridgeProxy);
    }

    function _deployFactory(address bridge) private returns (address) {
        CrossChainERC20Factory xChainERC20FactoryImpl = new CrossChainERC20Factory();
        CrossChainERC20Factory xChainERC20Factory = CrossChainERC20Factory(
            ERC1967Factory(ERC1967FactoryConstants.ADDRESS).deployAndCall({
                implementation: address(xChainERC20FactoryImpl),
                admin: PROXY_ADMIN,
                data: abi.encodeCall(CrossChainERC20Factory.initialize, (bridge))
            })
        );

        return address(xChainERC20Factory);
    }

    function _record(string memory out, string memory key, address addr, bool isLast)
        private
        pure
        returns (string memory)
    {
        return string.concat(out, "\"", key, "\": \"", vm.toString(addr), isLast ? "\"" : "\",");
    }

    function _addressToBytes32(address value) private pure returns (bytes32) {
        return bytes32(uint256(uint160(value)));
    }
}
