// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {EOA} from "optimism/packages/contracts-bedrock/src/libraries/EOA.sol";
import {Initializable} from "solady/utils/Initializable.sol";
import {SafeTransferLib} from "solady/utils/SafeTransferLib.sol";

import {CrossChainERC20} from "./CrossChainERC20.sol";
import {CrossChainMessenger} from "./CrossChainMessenger.sol";
import {MessagePasser} from "./MessagePasser.sol";
import {Encoder} from "./libraries/Encoder.sol";

contract Bridge is Initializable {
    //////////////////////////////////////////////////////////////
    ///                       Structs                          ///
    //////////////////////////////////////////////////////////////

    /// @notice Struct representing a bridge payload.
    ///
    /// @custom:field localToken Address of the ERC20 on this chain.
    /// @custom:field remoteToken Address of the corresponding token on the remote chain.
    /// @custom:field from Address of the sender.
    /// @custom:field to Address of the receiver.
    /// @custom:field amount Amount of the ERC20 being bridged.
    /// @custom:field extraData Extra data to be sent with the transaction. Note that the recipient will not be
    ///                         triggered with this data, but it will be emitted and can be used to identify the
    ///                         transaction.
    struct BridgePayload {
        bytes32 localToken;
        address remoteToken;
        address from;
        bytes32 to;
        uint64 amount;
        bytes extraData;
    }

    //////////////////////////////////////////////////////////////
    ///                       Events                           ///
    //////////////////////////////////////////////////////////////

    /// @notice Emitted when a token bridge is finalized.
    ///
    /// @param localToken Address of the ERC20 on this chain.
    /// @param remoteToken Address of the corresponding token on the remote chain.
    /// @param from Address of the sender.
    /// @param to Address of the receiver.
    /// @param amount Amount of the ERC20 being bridged.
    /// @param extraData Extra data to be sent with the transaction. Note that the recipient will not be triggered with
    ///                  this data, but it will be emitted and can be used to identify the transaction.
    event TokenBridgeFinalized(
        address localToken, bytes32 remoteToken, bytes32 from, address to, uint256 amount, bytes extraData
    );

    /// @notice Emitted when an ERC20 bridge is initiated to the other chain.
    /// @param localToken  Address of the ERC20 on this chain.
    /// @param remoteToken Address of the ERC20 on the remote chain.
    /// @param from        Address of the sender.
    /// @param to          Address of the receiver.
    /// @param amount      Amount of the ERC20 sent.
    /// @param extraData   Extra data sent with the transaction.
    event ERC20BridgeInitiated(
        address indexed localToken,
        bytes32 indexed remoteToken,
        address indexed from,
        bytes32 to,
        uint256 amount,
        bytes extraData
    );

    //////////////////////////////////////////////////////////////
    ///                       Storage                          ///
    //////////////////////////////////////////////////////////////

    /// @notice Messenger contract on this chain.
    address public messenger;

    /// @notice Bridge contract on the remote chain.
    ///
    /// @dev Stored as a bytes32 to handle non EVM addresses which may not fit into 20 bytes.
    bytes32 public remoteBridge;

    /// @notice Mapping that stores deposits for a given pair of local and remote tokens.
    mapping(address localToken => mapping(bytes32 remoteToken => uint256 amount)) public deposits;

    //////////////////////////////////////////////////////////////
    ///                       Modifiers                        ///
    //////////////////////////////////////////////////////////////

    /// @notice Only allow EOAs to call the functions. Note that this is not safe against contracts
    ///         calling code within their constructors, but also doesn't really matter since we're
    ///         just trying to prevent users accidentally depositing with smart contract wallets.
    modifier onlyEOA() {
        require(EOA.isSenderEOA(), "StandardBridge: function can only be called from an EOA");
        _;
    }

    /// @notice Ensures that the caller is the bridge on the remote chain.
    modifier onlyRemoteBridge() {
        require(
            msg.sender == messenger && CrossChainMessenger(messenger).xChainMsgSender() == remoteBridge,
            "Bridge: function can only be called from the other bridge"
        );
        _;
    }

    //////////////////////////////////////////////////////////////
    ///                       Public Functions                 ///
    //////////////////////////////////////////////////////////////

    /// @notice Constructs the Bridge contract.
    constructor() {
        _disableInitializers();
    }

    /// @notice Initializer.
    ///
    /// @param messenger_ Messenger contract on this chain.
    /// @param remoteBridge_ Bridge contract on the remote chain.
    function initialize(address messenger_, bytes32 remoteBridge_) external initializer {
        messenger = messenger_;
        remoteBridge = remoteBridge_;
    }

    /// @notice Sends ERC20 tokens to the sender's address on the other chain.
    /// @param localToken  Address of the ERC20 on this chain.
    /// @param remoteToken Address of the corresponding token on the remote chain.
    /// @param to          Solana pubkey to send tokens to
    /// @param amount      Amount of local tokens to deposit.
    /// @param extraData   Extra data to be sent with the transaction. Note that the recipient will
    ///                     not be triggered with this data, but it will be emitted and can be used
    ///                     to identify the transaction.
    function bridgeToken(address localToken, bytes32 remoteToken, bytes32 to, uint64 amount, bytes calldata extraData)
        public
        virtual
        onlyEOA
    {
        _initiateBridgeERC20(localToken, remoteToken, msg.sender, to, amount, extraData);
    }

    /// @notice Finalizes a token bridge on this chain. Can only be triggered by the Bridge contract on the remote
    ///         chain.
    ///
    /// @param localToken Address of the ERC20 on this chain.
    /// @param remoteToken Address of the corresponding token on the remote chain.
    /// @param from Address of the sender.
    /// @param to Address of the receiver.
    /// @param amount Amount of the ERC20 being bridged.
    /// @param extraData Extra data to be sent with the transaction. Note that the recipient will not be triggered with
    ///                  this data, but it will be emitted and can be used to identify the transaction.
    function finalizeBridgeToken(
        address localToken,
        bytes32 remoteToken,
        bytes32 from,
        address to,
        uint256 amount,
        bytes calldata extraData
    ) public onlyRemoteBridge {
        if (_isCrossChainERC20(localToken)) {
            CrossChainERC20 localToken_ = CrossChainERC20(localToken);

            require(
                _isCorrectTokenPair({localToken: localToken_, remoteToken: remoteToken}),
                "Bridge: wrong remote token for Optimism Mintable ERC20 local token"
            );

            localToken_.mint(to, amount);
        } else {
            deposits[localToken][remoteToken] = deposits[localToken][remoteToken] - amount;
            SafeTransferLib.safeTransfer({token: localToken, to: to, amount: amount});
        }

        emit TokenBridgeFinalized({
            localToken: localToken,
            remoteToken: remoteToken,
            from: from,
            to: to,
            amount: amount,
            extraData: extraData
        });
    }

    //////////////////////////////////////////////////////////////
    ///                       Internal Functions               ///
    //////////////////////////////////////////////////////////////

    /// @notice Sends ERC20 tokens to a receiver's address on the other chain.
    /// @param localToken  Address of the ERC20 on this chain.
    /// @param remoteToken Address of the corresponding token on the remote chain.
    /// @param to          Address of the receiver.
    /// @param amount      Amount of local tokens to deposit.
    /// @param extraData   Extra data to be sent with the transaction. Note that the recipient will
    ///                     not be triggered with this data, but it will be emitted and can be used
    ///                     to identify the transaction.
    function _initiateBridgeERC20(
        address localToken,
        bytes32 remoteToken,
        address from,
        bytes32 to,
        uint64 amount,
        bytes memory extraData
    ) internal {
        require(msg.value == 0, "StandardBridge: cannot send value");

        if (_isCrossChainERC20(localToken)) {
            require(
                _isCorrectTokenPair(CrossChainERC20(localToken), remoteToken),
                "StandardBridge: wrong remote token for Optimism Mintable ERC20 local token"
            );

            CrossChainERC20(localToken).burn(from, amount);
        } else {
            SafeTransferLib.safeTransferFrom({token: localToken, from: from, to: address(this), amount: amount});
            deposits[localToken][remoteToken] = deposits[localToken][remoteToken] + amount;
        }

        emit ERC20BridgeInitiated(localToken, remoteToken, from, to, amount, extraData);

        MessagePasser.Instruction[] memory messageIxs = new MessagePasser.Instruction[](1);
        messageIxs[0] = MessagePasser.Instruction({
            programId: remoteBridge,
            accounts: new MessagePasser.AccountMeta[](0),
            data: Encoder.encodeBridgePayload(
                BridgePayload({
                    localToken: remoteToken,
                    remoteToken: localToken,
                    from: from,
                    to: to,
                    amount: amount,
                    extraData: extraData
                })
            )
        });

        CrossChainMessenger(messenger).sendMessage(messageIxs);
    }

    /// @notice Checks if a given address is an CrossChainERC20. Not perfect, but good enough. Just the way we like it.
    ///
    /// @param token Address of the token to check.
    ///
    /// @return True if the token is an CrossChainERC20.
    function _isCrossChainERC20(address token) internal view returns (bool) {
        (bool success, bytes memory data) = token.staticcall(abi.encodeCall(CrossChainERC20.remoteToken, ()));
        return success && data.length == 32;
    }

    /// @notice Checks if the remote token is the correct pair token for the CrossChainERC20.
    ///
    /// @param localToken CrossChainERC20 to check against.
    /// @param remoteToken Pair token to check.
    ///
    /// @return True if the remote token is the correct pair token for the CrossChainERC20.
    function _isCorrectTokenPair(CrossChainERC20 localToken, bytes32 remoteToken) internal view returns (bool) {
        return localToken.remoteToken() == remoteToken;
    }
}
