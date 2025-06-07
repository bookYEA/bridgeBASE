// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {ERC20} from "solady/tokens/ERC20.sol";
import {SafeTransferLib} from "solady/utils/SafeTransferLib.sol";

import {Ix, Pubkey, SVMLib} from "./libraries/SVMLib.sol";
import {SVMTokenBridgeLib} from "./libraries/SVMTokenBridgeLib.sol";

import {CrossChainERC20} from "./CrossChainERC20.sol";

import {MessagePasser} from "./MessagePasser.sol";
import {Messenger} from "./Messenger.sol";

/// @title TokenBridge
///
/// @notice A cross-chain token bridge contract that facilitates token transfers between EVM-compatible chains and
/// Solana.
///         Supports both native tokens (ETH) and ERC20 tokens, including CrossChainERC20 tokens that can be
///         minted/burned on demand. Uses a messenger system to communicate with the corresponding bridge on
///         Solana.
///
/// @dev This contract is initializable and designed to work with EOAs only for security purposes.
///      It maintains deposit balances for standard tokens and handles minting/burning for CrossChainERC20 tokens.
contract TokenBridge {
    //////////////////////////////////////////////////////////////
    ///                       Events                           ///
    //////////////////////////////////////////////////////////////

    /// @notice Emitted when an ERC20 bridge transaction is initiated from this chain to Solana.
    ///
    /// @param localToken Address of the ERC20 token on this chain.
    /// @param remoteToken Pubkey of the remote token on Solana.
    /// @param from Address of the sender on this chain.
    /// @param to Pubkey of the intended recipient on Solana.
    /// @param amount Amount of tokens being bridged (expressed in EVM units).
    /// @param extraData Additional data sent with the transaction for identification purposes.
    event TokenBridgeInitiated(
        address indexed localToken,
        Pubkey indexed remoteToken,
        address indexed from,
        Pubkey to,
        uint256 amount,
        bytes extraData
    );

    /// @notice Emitted when a token bridge transaction is finalized on this chain.
    ///
    /// @param localToken Address of the ERC20 token on this chain.
    /// @param remoteToken Pubkey of the remote token on Solana.
    /// @param from Pubkey of the original sender on Solana.
    /// @param to Address of the recipient on this chain.
    /// @param amount Amount of tokens transferred to the recipient (expressed in EVM units).
    /// @param extraData Additional data associated with the bridge transaction.
    event TokenBridgeFinalized(
        address localToken, Pubkey remoteToken, Pubkey from, address to, uint256 amount, bytes extraData
    );

    //////////////////////////////////////////////////////////////
    ///                       Errors                           ///
    //////////////////////////////////////////////////////////////

    /// @notice Thrown when the ETH value sent with a transaction doesn't match the expected amount.
    error InvalidMsgValue();

    /// @notice Thrown when the remote token is not the expected token.
    error IncorrectRemoteToken();

    /// @notice Thrown when the token pair is not correct.
    error NotRemoteBridge();

    //////////////////////////////////////////////////////////////
    ///                       Constants                        ///
    //////////////////////////////////////////////////////////////

    /// @notice The ERC-7528 standard address representing native ETH in token operations.
    address public constant ETH_ADDRESS = 0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE;

    /// @notice The Solana pubkey for the native SOL token ("SoL1111111111111111111111111111111111111111")
    Pubkey public constant NATIVE_SOL_PUBKEY =
        Pubkey.wrap(0x0000000000000000000000000000000000000000000000000000000000000000);

    /// @notice Address of the Messenger contract on this chain.
    address public immutable MESSENGER;

    /// @notice Address of the MessagePasser contract on this chain.
    address public immutable MESSAGE_PASSER;

    /// @notice Pubkey of the token bridge contract on Solana.
    Pubkey public immutable REMOTE_TOKEN_BRIDGE;

    /// @notice The maximum number of decimals for a Solana token.
    uint8 public constant MAX_SOL_DECIMALS = 9;

    //////////////////////////////////////////////////////////////
    ///                       Storage                          ///
    //////////////////////////////////////////////////////////////

    /// @notice Mapping that stores deposit balances for token pairs between local and remote chains.
    ///
    /// @dev For standard ERC20 tokens and ETH, this tracks the total amount deposited and available for withdrawal.
    ///      CrossChainERC20 tokens bypass this mechanism as they use mint/burn instead.
    mapping(address localToken => mapping(Pubkey remoteToken => uint256 amount)) public deposits;

    //////////////////////////////////////////////////////////////
    ///                       Modifiers                        ///
    //////////////////////////////////////////////////////////////

    /// @notice Restricts function access to the bridge contract on Solana.
    ///
    /// @dev Ensures that only legitimate cross-chain messages from the paired bridge can trigger
    ///      finalization functions. Validates both the immediate caller (messenger) and the
    ///      original cross-chain message sender.
    modifier onlyRemoteBridge() {
        require(
            msg.sender == MESSENGER && Pubkey.wrap(Messenger(MESSENGER).xChainMsgSender()) == REMOTE_TOKEN_BRIDGE,
            NotRemoteBridge()
        );
        _;
    }

    //////////////////////////////////////////////////////////////
    ///                       Public Functions                 ///
    //////////////////////////////////////////////////////////////

    /// @notice Constructs the Bridge contract.
    ///
    /// @param messenger Address of the Messenger contract on this chain.
    /// @param messagePasser Address of the MessagePasser contract on this chain.
    /// @param remoteTokenBridge Pubkey of the token bridge contract on Solana.
    constructor(address messenger, address messagePasser, Pubkey remoteTokenBridge) {
        MESSENGER = messenger;
        MESSAGE_PASSER = messagePasser;
        REMOTE_TOKEN_BRIDGE = remoteTokenBridge;
    }

    /// @notice Bridges a token to Solana.
    ///
    /// @param localToken Address of the ERC20 token on this chain.
    /// @param remoteToken Pubkey of the remote token on Solana.
    /// @param to Pubkey of the intended recipient on Solana.
    /// @param remoteAmount Amount of tokens being bridged (expressed in Solana units).
    /// @param extraData Additional data sent with the transaction for identification purposes.
    function bridgeToken(
        address localToken,
        Pubkey remoteToken,
        Pubkey to,
        uint64 remoteAmount,
        bytes calldata extraData
    ) external payable {
        uint256 evmAmount;
        Ix memory ix;

        if (localToken == ETH_ADDRESS) {
            // Case: Bridging native ETH to Solana
            uint8 remoteDecimals;
            (evmAmount, remoteDecimals) = _localAmount({remoteAmount: remoteAmount, localDecimals: 18});
            require(msg.value == evmAmount, InvalidMsgValue());

            deposits[localToken][remoteToken] += evmAmount;

            ix = SVMTokenBridgeLib.finalizeBridgeTokenIx({
                remoteBridge: REMOTE_TOKEN_BRIDGE,
                localToken: localToken,
                remoteToken: remoteToken,
                to: to,
                remoteAmount: remoteAmount
            });
        } else {
            uint8 remoteDecimals;
            (evmAmount, remoteDecimals) =
                _localAmount({remoteAmount: remoteAmount, localDecimals: ERC20(localToken).decimals()});

            try CrossChainERC20(localToken).remoteToken() returns (bytes32 remoteToken_) {
                // Case: Bridging back native SOL or SPL token to Solana
                require(Pubkey.wrap(remoteToken_) == remoteToken, IncorrectRemoteToken());
                CrossChainERC20(localToken).burn({from: msg.sender, amount: evmAmount});

                ix = remoteToken == NATIVE_SOL_PUBKEY
                    ? SVMTokenBridgeLib.finalizeBridgeSolIx({
                        remoteBridge: REMOTE_TOKEN_BRIDGE,
                        localToken: localToken,
                        to: to,
                        remoteAmount: remoteAmount
                    })
                    : SVMTokenBridgeLib.finalizeBridgeSplIx({
                        remoteBridge: REMOTE_TOKEN_BRIDGE,
                        localToken: localToken,
                        remoteToken: remoteToken,
                        to: to,
                        remoteAmount: remoteAmount
                    });
            } catch {
                // Case: Bridging native ERC20 to Solana
                SafeTransferLib.safeTransferFrom({
                    token: localToken,
                    from: msg.sender,
                    to: address(this),
                    amount: evmAmount
                });

                deposits[localToken][remoteToken] += evmAmount;

                ix = SVMTokenBridgeLib.finalizeBridgeTokenIx({
                    remoteBridge: REMOTE_TOKEN_BRIDGE,
                    localToken: localToken,
                    remoteToken: remoteToken,
                    to: to,
                    remoteAmount: remoteAmount
                });
            }
        }

        Ix[] memory ixs = new Ix[](1);
        ixs[0] = ix;
        MessagePasser(MESSENGER).sendRemoteCall(SVMLib.serializeAnchorIxs(ixs));

        emit TokenBridgeInitiated({
            localToken: localToken,
            remoteToken: remoteToken,
            from: msg.sender,
            to: to,
            amount: evmAmount,
            extraData: extraData
        });
    }

    /// @notice Finalizes a token bridge transaction initiated from Solana.
    ///
    /// @dev This function can only be called by the remote bridge through the messenger system. For CrossChainERC20
    ///      tokens, it mints new tokens. For standard tokens, it withdraws from the deposit pool. Supports both
    ///      ERC20 tokens and native ETH.
    ///
    /// @param localToken Address of the ERC20 token on this chain (use ETH_ADDRESS for native ETH).
    /// @param remoteToken Pubkey of the remote token on Solana.
    /// @param from Pubkey of the original sender on Solana.
    /// @param to Address of the recipient on this chain.
    /// @param remoteAmount Amount of tokens being bridged from Solana (expressed in Solana units).
    /// @param extraData Additional data associated with the original bridge transaction.
    function finalizeBridgeToken(
        address localToken,
        Pubkey remoteToken,
        Pubkey from,
        address to,
        uint64 remoteAmount,
        bytes calldata extraData
    ) public onlyRemoteBridge {
        uint256 evmAmount;
        if (localToken == ETH_ADDRESS) {
            // Case: Bridging back native ETH to EVM
            uint8 remoteDecimals;
            (evmAmount, remoteDecimals) = _localAmount({remoteAmount: remoteAmount, localDecimals: 18});
            deposits[localToken][remoteToken] -= evmAmount;

            SafeTransferLib.safeTransferETH({to: to, amount: evmAmount});
        } else {
            uint8 remoteDecimals;
            (evmAmount, remoteDecimals) =
                _localAmount({remoteAmount: remoteAmount, localDecimals: ERC20(localToken).decimals()});

            try CrossChainERC20(localToken).remoteToken() returns (bytes32 remoteToken_) {
                // Case: Bridging native SOL or SPL token to EVM
                require(Pubkey.wrap(remoteToken_) == remoteToken, IncorrectRemoteToken());
                CrossChainERC20(localToken).mint({to: to, amount: evmAmount});
            } catch {
                // Case: Bridging back native ERC20 to EVM
                deposits[localToken][remoteToken] -= evmAmount;
                SafeTransferLib.safeTransfer({token: localToken, to: to, amount: evmAmount});
            }
        }

        emit TokenBridgeFinalized({
            localToken: localToken,
            remoteToken: remoteToken,
            from: from,
            to: to,
            amount: evmAmount,
            extraData: extraData
        });
    }

    //////////////////////////////////////////////////////////////
    ///                       Private Functions                ///
    //////////////////////////////////////////////////////////////

    /// @notice Converts a remote amount to a local amount.
    ///
    /// @param remoteAmount Amount of tokens being bridged to/from Solana.
    /// @param localDecimals Number of decimals on this chain.
    ///
    /// @return localAmount Amount of tokens in the EVM chain.
    /// @return remoteDecimals Number of decimals to be used on Solana.
    function _localAmount(uint64 remoteAmount, uint8 localDecimals)
        internal
        pure
        returns (uint256 localAmount, uint8 remoteDecimals)
    {
        remoteDecimals = localDecimals;
        localAmount = remoteAmount;

        if (localDecimals > MAX_SOL_DECIMALS) {
            remoteDecimals = MAX_SOL_DECIMALS;
            uint256 scaleFactor = 10 ** (localDecimals - remoteDecimals);
            localAmount = remoteAmount * scaleFactor;
        }
    }
}
