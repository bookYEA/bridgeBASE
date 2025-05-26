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
import {Encoder} from "../src/libraries/Encoder.sol";

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
            new CrossChainMessenger(MessagePasser(payable(address(messagePasser))), remoteMessenger);
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

    function test_encoder() external pure {
        uint256 nonce = 0;
        address sender = 0x22B66c7FBC67f57d16FC94e862902098CD7b2972;
        MessagePasser.Instruction[] memory ixs = new MessagePasser.Instruction[](1);
        MessagePasser.AccountMeta[] memory accounts = new MessagePasser.AccountMeta[](0);
        bytes memory data =
            hex"5974ca4b16cafc5c28f2b4227f26f8c58fd70559312c4f612f7866280e394265e398d7afe84a6339783718935087a4ace6f6dfe8000102030405060708090a0b0c0d0e0f1011121301a2dac43f8b87f394d468ef6842e6360a980ebeb99e8df71faa5daab591adf700ca9a3b000000000b00000072616e646f6d2064617461";
        ixs[0] = MessagePasser.Instruction({
            programId: 0x7a25452c36304317d6fe970091c383b0d45e9b0b06485d2561156f025c6936af,
            accounts: accounts,
            data: data
        });
        CrossChainMessenger.MessengerPayload memory payload =
            CrossChainMessenger.MessengerPayload({nonce: nonce, sender: sender, ixs: ixs});

        assertEq(
            Encoder.encodeMessengerPayload(payload),
            hex"000000000000000000000000000000000000000000000000000000000000000022b66c7fbc67f57d16fc94e862902098cd7b2972ab000000010000007a25452c36304317d6fe970091c383b0d45e9b0b06485d2561156f025c6936af000000007f0000005974ca4b16cafc5c28f2b4227f26f8c58fd70559312c4f612f7866280e394265e398d7afe84a6339783718935087a4ace6f6dfe8000102030405060708090a0b0c0d0e0f1011121301a2dac43f8b87f394d468ef6842e6360a980ebeb99e8df71faa5daab591adf700ca9a3b000000000b00000072616e646f6d2064617461"
        );
    }

    function test_encoder_bridgePayload() external pure {
        bytes32 localToken = 0x2b2d21b7ff083222845f554b5c1f2e1e4affd92e7597f9f4e9b63e6c369d4cb7;
        address remoteToken_ = 0xE398D7afe84A6339783718935087a4AcE6F6DFE8;
        address from = 0x000102030405060708090a0b0c0d0e0f10111213;
        bytes32 to = 0xacd56258cfa53dc77d9290116210958c82bf2fc115f1f7f392e530aba3a03fb3;
        uint64 amount = 0x00ca9a3b00000000;
        bytes memory extraData = hex"72616e646f6d2064617461";
        Bridge.BridgePayload memory payload = Bridge.BridgePayload({
            localToken: localToken,
            remoteToken: remoteToken_,
            from: from,
            to: to,
            amount: amount,
            extraData: extraData
        });

        assertEq(
            Encoder.encodeBridgePayload(payload),
            hex"2b2d21b7ff083222845f554b5c1f2e1e4affd92e7597f9f4e9b63e6c369d4cb7e398d7afe84a6339783718935087a4ace6f6dfe8000102030405060708090a0b0c0d0e0f10111213acd56258cfa53dc77d9290116210958c82bf2fc115f1f7f392e530aba3a03fb3000000003b9aca000b00000072616e646f6d2064617461"
        );
    }

    function _bytes32ToAddress(bytes32 value) private pure returns (address) {
        return address(uint160(uint256(value)));
    }
}
