// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {Initializable} from "solady/utils/Initializable.sol";
import {SafeTransferLib} from "solady/utils/SafeTransferLib.sol";

import {CrossChainMessenger} from "./CrossChainMessenger.sol";
import {ICrossChainERC20} from "./interfaces/ICrossChainERC20.sol";

/// @custom:upgradeable
/// @title Bridge
contract Bridge is Initializable {
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
        address localToken, address remoteToken, address from, address to, uint256 amount, bytes extraData
    );

    //////////////////////////////////////////////////////////////
    ///                       Storage                          ///
    //////////////////////////////////////////////////////////////

    /// @notice Messenger contract on this chain.
    address public messenger;

    /// @notice Bridge contract on the remote chain.
    address public remoteBridge;

    /// @notice Mapping that stores deposits for a given pair of local and remote tokens.
    mapping(address localToken => mapping(address remoteToken => uint256 amount)) public deposits;

    //////////////////////////////////////////////////////////////
    ///                       Modifiers                        ///
    //////////////////////////////////////////////////////////////

    /// @notice Ensures that the caller is the bridge on the remote chain.
    modifier onlyRemoteBridge() {
        require(
            msg.sender == messenger && CrossChainMessenger(messenger).xChainMsgSender() == address(remoteBridge),
            "Bridge: function can only be called from the other bridge"
        );
        _;
    }

    //////////////////////////////////////////////////////////////
    ///                       Public Functions                 ///
    //////////////////////////////////////////////////////////////

    /// @notice Initializer.
    ///
    /// @param messenger_ Messenger contract on this chain.
    /// @param remoteBridge_ Bridge contract on the remote chain.
    function initialize(address messenger_, address remoteBridge_) external initializer {
        messenger = messenger_;
        remoteBridge = remoteBridge_;
    }

    /// @notice Finalizes a token bridge on this chain. Can only be triggered by the other Bridge contract on
    ///         the remote chain.
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
        address remoteToken,
        address from,
        address to,
        uint256 amount,
        bytes calldata extraData
    ) public onlyRemoteBridge {
        if (_isCrossChainERC20(localToken)) {
            ICrossChainERC20 localToken_ = ICrossChainERC20(localToken);

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

    /// @notice Checks if a given address is an CrossChainERC20. Not perfect, but good enough. Just the way we like it.
    ///
    /// @param token Address of the token to check.
    ///
    /// @return True if the token is an CrossChainERC20.
    function _isCrossChainERC20(address token) internal view returns (bool) {
        return ICrossChainERC20(token).supportsInterface(type(ICrossChainERC20).interfaceId);
    }

    /// @notice Checks if the remote token is the correct pair token for the CrossChainERC20.
    ///
    /// @param localToken CrossChainERC20 to check against.
    /// @param remoteToken Pair token to check.
    ///
    /// @return True if the remote token is the correct pair token for the CrossChainERC20.
    function _isCorrectTokenPair(ICrossChainERC20 localToken, address remoteToken) internal view returns (bool) {
        return localToken.remoteToken() == remoteToken;
    }
}
