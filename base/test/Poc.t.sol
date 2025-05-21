// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {Test} from "forge-std/Test.sol";

import {ERC1967Factory} from "solady/utils/ERC1967Factory.sol";
import {ERC1967FactoryConstants} from "solady/utils/ERC1967FactoryConstants.sol";

import {Bridge} from "../src/Bridge.sol";
import {CrossChainERC20} from "../src/CrossChainERC20.sol";
import {CrossChainERC20Factory} from "../src/CrossChainERC20Factory.sol";
import {CrossChainMessenger} from "../src/CrossChainMessenger.sol";
import {MessagePasser} from "../src/MessagePasser.sol";
import {ISolanaMessagePasser} from "../src/interfaces/ISolanaMessagePasser.sol";

contract Poc is Test {
    address proxyAdmin;

    bytes32 remoteMessenger;
    bytes32 remoteBridge;
    bytes32 remoteToken;

    CrossChainMessenger messengerProxy;
    Bridge bridgeProxy;

    CrossChainERC20 cbSOL;

    function setUp() public {
        proxyAdmin = makeAddr("PROXY_ADMIN");

        remoteMessenger = bytes32(uint256(uint160(makeAddr("REMOTE_MESSENGER"))));
        remoteBridge = bytes32(uint256(uint160(makeAddr("REMOTE_BRIDGE"))));
        remoteToken = bytes32(uint256(uint160(makeAddr("REMOTE_TOKEN"))));

        // Deploy the ERC1967Factory
        vm.etch(ERC1967FactoryConstants.ADDRESS, ERC1967FactoryConstants.BYTECODE);

        // Deploy the SolanaMessagePasser
        MessagePasser messagePasser = new MessagePasser();

        // Deploy the CrossChainMessenger
        CrossChainMessenger messengerImpl =
            new CrossChainMessenger(ISolanaMessagePasser(payable(address(messagePasser))), remoteMessenger);
        messengerProxy = CrossChainMessenger(
            ERC1967Factory(ERC1967FactoryConstants.ADDRESS).deployAndCall({
                implementation: address(messengerImpl),
                admin: proxyAdmin,
                data: abi.encodeCall(CrossChainMessenger.initialize, (remoteMessenger))
            })
        );

        // Deploy the Bridge
        Bridge bridgeImpl = new Bridge();
        bridgeProxy = Bridge(
            ERC1967Factory(ERC1967FactoryConstants.ADDRESS).deployAndCall({
                implementation: address(bridgeImpl),
                admin: proxyAdmin,
                data: abi.encodeCall(Bridge.initialize, (address(messengerProxy), remoteBridge))
            })
        );

        // Deploy the CrossChainERC20Factory
        CrossChainERC20Factory xChainERC20FactoryImpl = new CrossChainERC20Factory();
        CrossChainERC20Factory xChainERC20Factory = CrossChainERC20Factory(
            ERC1967Factory(ERC1967FactoryConstants.ADDRESS).deployAndCall({
                implementation: address(xChainERC20FactoryImpl),
                admin: proxyAdmin,
                data: abi.encodeCall(CrossChainERC20Factory.initialize, (address(bridgeProxy)))
            })
        );

        // Deploy the CrossChainERC20
        cbSOL = CrossChainERC20(
            xChainERC20Factory.deploy({remoteToken: remoteToken, name: "cbSOL", symbol: "cbSOL", decimals: 9})
        );
    }

    function testTokenBridge() public {
        bytes32 from = bytes32(uint256(uint160(makeAddr("ALICE_SOL"))));
        address to = makeAddr("ALICE_BASE");
        uint256 amount = 42;
        bytes memory extraData = "";

        vm.prank(_bytes32ToAddress(remoteMessenger));
        messengerProxy.relayMessage({
            nonce: 0,
            sender: remoteBridge,
            target: address(bridgeProxy),
            value: 0,
            minGasLimit: 0,
            message: abi.encodeCall(Bridge.finalizeBridgeToken, (address(cbSOL), remoteToken, from, to, amount, extraData))
        });

        vm.assertEq(cbSOL.balanceOf(to), amount);
    }

    function _bytes32ToAddress(bytes32 value) private pure returns (address) {
        return address(uint160(uint256(value)));
    }
}
