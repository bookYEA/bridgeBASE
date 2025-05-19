// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {EOA} from "optimism/packages/contracts-bedrock/src/libraries/EOA.sol";
import {Initializable} from "solady/utils/Initializable.sol";
import {SafeTransferLib} from "solady/utils/SafeTransferLib.sol";

import {CrossChainERC20} from "./CrossChainERC20.sol";
import {CrossChainMessenger} from "./CrossChainMessenger.sol";
import {ISolanaMessagePasser} from "./interfaces/ISolanaMessagePasser.sol";

contract Bridge is Initializable {
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
    /// @param _localToken  Address of the ERC20 on this chain.
    /// @param _remoteToken Address of the corresponding token on the remote chain.
    /// @param _to          Solana pubkey to send tokens to
    /// @param _amount      Amount of local tokens to deposit.
    /// @param _extraData   Extra data to be sent with the transaction. Note that the recipient will
    ///                     not be triggered with this data, but it will be emitted and can be used
    ///                     to identify the transaction.
    function bridgeToken(
        address _localToken,
        bytes32 _remoteToken,
        bytes32 _to,
        uint64 _amount,
        bytes calldata _extraData
    ) public virtual onlyEOA {
        _initiateBridgeERC20(_localToken, _remoteToken, msg.sender, _to, _amount, _extraData);
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
    /// @param _localToken  Address of the ERC20 on this chain.
    /// @param _remoteToken Address of the corresponding token on the remote chain.
    /// @param _to          Address of the receiver.
    /// @param _amount      Amount of local tokens to deposit.
    /// @param _extraData   Extra data to be sent with the transaction. Note that the recipient will
    ///                     not be triggered with this data, but it will be emitted and can be used
    ///                     to identify the transaction.
    function _initiateBridgeERC20(
        address _localToken,
        bytes32 _remoteToken,
        address _from,
        bytes32 _to,
        uint64 _amount,
        bytes memory _extraData
    ) internal {
        require(msg.value == 0, "StandardBridge: cannot send value");

        if (_isCrossChainERC20(_localToken)) {
            require(
                _isCorrectTokenPair(CrossChainERC20(_localToken), _remoteToken),
                "StandardBridge: wrong remote token for Optimism Mintable ERC20 local token"
            );

            CrossChainERC20(_localToken).burn(_from, _amount);
        } else {
            SafeTransferLib.safeTransferFrom({token: _localToken, from: _from, to: address(this), amount: _amount});
            deposits[_localToken][_remoteToken] = deposits[_localToken][_remoteToken] + _amount;
        }

        emit ERC20BridgeInitiated(_localToken, _remoteToken, _from, _to, _amount, _extraData);

        ISolanaMessagePasser.Instruction[] memory messageIxs = new ISolanaMessagePasser.Instruction[](1);
        messageIxs[0] = ISolanaMessagePasser.Instruction({
            programId: remoteBridge,
            accounts: new ISolanaMessagePasser.AccountMeta[](0),
            data: abi.encode(
                BridgePayload({
                    localToken: _remoteToken,
                    remoteToken: _localToken,
                    from: _from,
                    to: _to,
                    amount: _amount,
                    extraData: _extraData
                })
            )
        });

        CrossChainMessenger(messenger).sendMessage({messageIxs: messageIxs});
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
