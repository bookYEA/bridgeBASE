// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {SafeTransferLib} from "solady/utils/SafeTransferLib.sol";

import {CrossChainERC20} from "../CrossChainERC20.sol";

import {MessageStorageLib} from "./MessageStorageLib.sol";
import {Ix, Pubkey, SVMLib} from "./SVMLib.sol";
import {SVMTokenBridgeLib} from "./SVMTokenBridgeLib.sol";

/// @notice Struct representing a transfer payload.
///
/// @custom:field localToken Address of the ERC20 token on this chain.
/// @custom:field remoteToken Pubkey of the remote token on Solana.
/// @custom:field to Address of the recipient on the target chain. EVM address on Base, Solana pubkey on Solana.
/// @custom:field remoteAmount Amount of tokens being bridged (expressed in Solana units).
struct TransferPayload {
    address localToken;
    Pubkey remoteToken;
    bytes32 to;
    uint64 remoteAmount;
}

/// @notice Storage layout used by this contract.
///
/// @custom:storage-location erc7201:coinbase.storage.TokenLib
///
/// @custom:field deposits Mapping that stores deposit balances for token pairs between Base and Solana.
/// @custom:field scalars Mapping that stores the scalars to use to scale Solana amounts to Base amounts.
///                               Only used when bridging native ETH or ERC20 tokens to or back from Solana.
struct TokenLibStorage {
    mapping(address localToken => mapping(Pubkey remoteToken => uint256 amount)) deposits;
    mapping(address localToken => mapping(Pubkey remoteToken => uint256 scaler)) scalars;
}

library TokenLib {
    //////////////////////////////////////////////////////////////
    ///                       Errors                           ///
    //////////////////////////////////////////////////////////////

    /// @notice Thrown when the ETH value sent with a transaction doesn't match the expected amount.
    error InvalidMsgValue();

    /// @notice Thrown when the remote token is not the expected token.
    error IncorrectRemoteToken();

    /// @notice Thrown when the token pair is not correct.
    error NotRemoteBridge();

    /// @notice Thrown when the token pair is not registered.
    error WrappedSplRouteNotRegistered();

    //////////////////////////////////////////////////////////////
    ///                       Events                           ///
    //////////////////////////////////////////////////////////////

    /// @notice Emitted when a token bridge transaction is finalized.
    ///
    /// @param localToken Address of the ERC20 token.
    /// @param remoteToken Pubkey of the remote token on Solana.
    /// @param to Address of the recipient on.
    /// @param amount Amount of tokens transferred to the recipient (expressed in EVM units).
    event TokenBridgeFinalized(address localToken, Pubkey remoteToken, address to, uint256 amount);

    //////////////////////////////////////////////////////////////
    ///                       Constants                        ///
    //////////////////////////////////////////////////////////////

    /// @notice The ERC-7528 standard address representing native ETH in token operations.
    address public constant ETH_ADDRESS = 0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE;

    /// @notice The Solana pubkey for the native SOL token ("SoL1111111111111111111111111111111111111111")
    Pubkey public constant NATIVE_SOL_PUBKEY =
        Pubkey.wrap(0x069be72ab836d4eacc02525b7350a78a395da2f1253a40ebafd6630000000000);

    /// @notice Pubkey of the token bridge contract on Solana.
    Pubkey public constant REMOTE_TOKEN_BRIDGE =
        Pubkey.wrap(0x0000000000000000000000000000000000000000000000000000000000000000);

    /// @dev Slot for the `TokenLibStorage` struct in storage.
    ///      Computed from:
    ///         keccak256(abi.encode(uint256(keccak256("coinbase.storage.TokenLib")) - 1)) & ~bytes32(uint256(0xff))
    ///
    ///      Follows ERC-7201 (see https://eips.ethereum.org/EIPS/eip-7201).
    bytes32 private constant _TOKEN_LIB_STORAGE_LOCATION =
        0x86fd1c0757ed9526a07041356cbdd3c36e2a83be313529de06f943db14148300;

    //////////////////////////////////////////////////////////////
    ///                       Internal Functions               ///
    //////////////////////////////////////////////////////////////

    /// @notice Helper function to get a storage reference to the `TokenLibStorage` struct.
    ///
    /// @return $ A storage reference to the `TokenLibStorage` struct.
    function getTokenLibStorage() internal pure returns (TokenLibStorage storage $) {
        assembly ("memory-safe") {
            $.slot := _TOKEN_LIB_STORAGE_LOCATION
        }
    }

    function initiateTransfer(TransferPayload memory payload) internal {
        Pubkey to = Pubkey.wrap(payload.to);
        TokenLibStorage storage $ = getTokenLibStorage();

        uint256 localAmount;
        Ix memory ix;

        if (payload.localToken == ETH_ADDRESS) {
            // Case: Bridging native ETH to Solana
            uint256 scaler = $.scalars[payload.localToken][payload.remoteToken];
            require(scaler != 0, WrappedSplRouteNotRegistered());

            localAmount = payload.remoteAmount * scaler;
            require(msg.value == localAmount, InvalidMsgValue());

            ix = SVMTokenBridgeLib.finalizeBridgeTokenIx({
                remoteBridge: REMOTE_TOKEN_BRIDGE,
                localToken: payload.localToken,
                remoteToken: payload.remoteToken,
                to: to,
                remoteAmount: payload.remoteAmount
            });
        } else {
            // Prevent sending ETH when bridging ERC20 tokens
            require(msg.value == 0, InvalidMsgValue());

            try CrossChainERC20(payload.localToken).remoteToken() returns (bytes32 remoteToken_) {
                // Case: Bridging back native SOL or SPL token to Solana
                require(Pubkey.wrap(remoteToken_) == payload.remoteToken, IncorrectRemoteToken());

                localAmount = payload.remoteAmount;
                CrossChainERC20(payload.localToken).burn({from: msg.sender, amount: localAmount});

                ix = payload.remoteToken == NATIVE_SOL_PUBKEY
                    ? SVMTokenBridgeLib.finalizeBridgeSolIx({
                        remoteBridge: REMOTE_TOKEN_BRIDGE,
                        localToken: payload.localToken,
                        to: to,
                        remoteAmount: payload.remoteAmount
                    })
                    : SVMTokenBridgeLib.finalizeBridgeSplIx({
                        remoteBridge: REMOTE_TOKEN_BRIDGE,
                        localToken: payload.localToken,
                        remoteToken: payload.remoteToken,
                        to: to,
                        remoteAmount: payload.remoteAmount
                    });
            } catch {
                // Case: Bridging native ERC20 to Solana
                uint256 scaler = $.scalars[payload.localToken][payload.remoteToken];
                require(scaler != 0, WrappedSplRouteNotRegistered());

                localAmount = payload.remoteAmount * scaler;

                SafeTransferLib.safeTransferFrom({
                    token: payload.localToken,
                    from: msg.sender,
                    to: address(this),
                    amount: localAmount
                });

                $.deposits[payload.localToken][payload.remoteToken] += localAmount;

                ix = SVMTokenBridgeLib.finalizeBridgeTokenIx({
                    remoteBridge: REMOTE_TOKEN_BRIDGE,
                    localToken: payload.localToken,
                    remoteToken: payload.remoteToken,
                    to: to,
                    remoteAmount: payload.remoteAmount
                });
            }
        }

        Ix[] memory ixs = new Ix[](1);
        ixs[0] = ix;
        MessageStorageLib.sendMessage({sender: msg.sender, data: SVMLib.serializeAnchorIxs(ixs)});
    }

    function finalizeTransfer(TransferPayload memory payload) internal {
        // TODO: Rather this or shift right?
        address to = address(bytes20(payload.to));

        uint256 localAmount;

        TokenLibStorage storage $ = getTokenLibStorage();

        if (payload.localToken == ETH_ADDRESS) {
            // Case: Bridging back native ETH to EVM
            uint256 scaler = $.scalars[payload.localToken][payload.remoteToken];
            require(scaler != 0, WrappedSplRouteNotRegistered());
            localAmount = payload.remoteAmount * scaler;

            SafeTransferLib.safeTransferETH({to: to, amount: localAmount});
        } else {
            try CrossChainERC20(payload.localToken).remoteToken() returns (bytes32 remoteToken_) {
                // Case: Bridging native SOL or SPL token to EVM
                require(Pubkey.wrap(remoteToken_) == payload.remoteToken, IncorrectRemoteToken());
                localAmount = payload.remoteAmount;
                CrossChainERC20(payload.localToken).mint({to: to, amount: localAmount});
            } catch {
                // Case: Bridging back native ERC20 to EVM
                uint256 scaler = $.scalars[payload.localToken][payload.remoteToken];
                require(scaler != 0, WrappedSplRouteNotRegistered());

                localAmount = payload.remoteAmount * scaler;
                $.deposits[payload.localToken][payload.remoteToken] -= localAmount;

                SafeTransferLib.safeTransfer({token: payload.localToken, to: to, amount: localAmount});
            }
        }

        emit TokenBridgeFinalized({
            localToken: payload.localToken,
            remoteToken: payload.remoteToken,
            to: to,
            amount: localAmount
        });
    }

    //////////////////////////////////////////////////////////////
    ///                       Private Functions                ///
    //////////////////////////////////////////////////////////////
}
