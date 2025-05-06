// SPDX-License-Identifier: MIT
pragma solidity 0.8.15;

import {Script} from "forge-std/Script.sol";

import {Strings} from "@openzeppelin/contracts/utils/Strings.sol";
import {L2CrossDomainMessenger, CrossDomainMessenger} from "optimism/packages/contracts-bedrock/src/L2/L2CrossDomainMessenger.sol";
import {L2StandardBridge, StandardBridge} from "optimism/packages/contracts-bedrock/src/L2/L2StandardBridge.sol";
import {OptimismMintableERC20Factory} from "optimism/packages/contracts-bedrock/src/universal/OptimismMintableERC20Factory.sol";
import {Proxy} from "optimism/packages/contracts-bedrock/src/universal/Proxy.sol";

contract DeployScript is Script {
    address public constant ORACLE = 0x0e9a877906EBc3b7098DA2404412BF0Ed1A5EFb4;
    address public immutable ADMIN;
    address public immutable OTHER_BRIDGE;

    constructor() {
        ADMIN = vm.envAddress("ADMIN");
        OTHER_BRIDGE = vm.envAddress("OTHER_BRIDGE");
    }

    function run() public {
        string memory out = "{";

        vm.startBroadcast();
        address messenger = _deployMessenger();
        out = _record(out, messenger, "L2CrossDomainMessenger");

        address bridge = _deployBridge(messenger);
        out = _record(out, bridge, "L2StandardBridge");

        address factory = _deployFactory(bridge);
        out = _record(out, factory, "OptimismMintableERC20Factory");
        vm.stopBroadcast();

        out = string.concat(out, "}");
        vm.writeFile(string.concat("deployments/", _getChainName(), ".json"), out);
    }

    function _deployMessenger() private returns (address) {
        bytes memory messengerCall = abi.encodeCall(L2CrossDomainMessenger.initialize, (CrossDomainMessenger(ORACLE)));

        L2CrossDomainMessenger messengerImpl = new L2CrossDomainMessenger();

        Proxy messengerProxy = new Proxy(ADMIN);
        messengerProxy.upgradeToAndCall(address(messengerImpl), messengerCall);
        return address(messengerProxy);
    }

    function _deployBridge(address messenger) private returns (address) {
        bytes memory bridgeCall = "";
        // bytes memory bridgeCall = abi.encodeCall(L2StandardBridge.initialize, (StandardBridge(payable(OTHER_BRIDGE)), messenger));

        L2StandardBridge l2BridgeImpl = new L2StandardBridge();

        Proxy bridgeProxy = new Proxy(ADMIN);
        bridgeProxy.upgradeToAndCall(address(l2BridgeImpl), bridgeCall);
        return address(bridgeProxy);
    }

    function _deployFactory(address bridge) private returns (address) {
        bytes memory factoryCall = abi.encodeCall(OptimismMintableERC20Factory.initialize, (bridge));

        OptimismMintableERC20Factory factoryImpl = new OptimismMintableERC20Factory();

        Proxy factoryProxy = new Proxy(ADMIN);
        factoryProxy.upgradeToAndCall(address(factoryImpl), factoryCall);
        return address(factoryProxy);
    }

    function _record(string memory out, address contractAddr, string memory key) private pure returns (string memory) {
        return string.concat(out, "\"", key, "\": \"", Strings.toHexString(contractAddr), "\",");
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
