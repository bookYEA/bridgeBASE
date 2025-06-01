// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {EOA} from "optimism/packages/contracts-bedrock/src/libraries/EOA.sol";
import {Initializable} from "solady/utils/Initializable.sol";
import {SafeTransferLib} from "solady/utils/SafeTransferLib.sol";

import {CrossChainERC20} from "./CrossChainERC20.sol";
import {CrossChainMessenger} from "./CrossChainMessenger.sol";
import {MessagePasser} from "./MessagePasser.sol";
import {Encoder} from "./libraries/Encoder.sol";

/// @title TokenBridge
///
/// @notice A cross-chain token bridge contract that facilitates token transfers between EVM-compatible chains and
/// Solana.
///         Supports both native tokens (ETH) and ERC20 tokens, including CrossChainERC20 tokens that can be
///         minted/burned on demand. Uses a messenger system to communicate with the corresponding bridge on
///         the remote chain.
///
/// @dev This contract is initializable and designed to work with EOAs only for security purposes.
///      It maintains deposit balances for standard tokens and handles minting/burning for CrossChainERC20 tokens.
contract TokenBridge is Initializable {
    //////////////////////////////////////////////////////////////
    ///                       Structs                          ///
    //////////////////////////////////////////////////////////////

    /// @notice Struct representing a bridge payload that gets encoded and sent to the remote chain.
    ///
    /// @dev Note that `localToken` and `remoteToken` seem to be swapped. This is on purpose since this payload is
    ///      handled in the context of the remote chain.
    ///
    /// @custom:field localToken  Address of the ERC20 token on the remote chain.
    /// @custom:field remoteToken Address/public key of the corresponding token on this chain.
    /// @custom:field from        Address of the sender on this chain.
    /// @custom:field to          Address/public key of the receiver on the remote chain.
    /// @custom:field amount      Amount of the token being bridged (uint64 for compatibility with Solana's token
    ///                           amounts).
    /// @custom:field extraData   Additional data to be sent with the transaction for identification and tracking
    ///                           purposes. The recipient will not be triggered with this data, but it will be emitted
    ///                           in events.
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

    /// @notice Emitted when a token bridge transaction is finalized on this chain.
    ///
    /// @param localToken  Address of the ERC20 token on this chain.
    /// @param remoteToken Address/public key of the corresponding token on the remote chain.
    /// @param from        Address/public key of the original sender on the remote chain.
    /// @param to          Address of the recipient on this chain.
    /// @param amount      Amount of tokens transferred to the recipient.
    /// @param extraData   Additional data associated with the bridge transaction.
    event TokenBridgeFinalized(
        address localToken, bytes32 remoteToken, bytes32 from, address to, uint256 amount, bytes extraData
    );

    /// @notice Emitted when an ERC20 bridge transaction is initiated from this chain to the remote chain.
    ///
    /// @param localToken  Address of the ERC20 token on this chain.
    /// @param remoteToken Address/public key of the corresponding token on the remote chain.
    /// @param from        Address of the sender on this chain.
    /// @param to          Address/public key of the intended recipient on the remote chain.
    /// @param amount      Amount of tokens being bridged.
    /// @param extraData   Additional data sent with the transaction for identification purposes.
    event ERC20BridgeInitiated(
        address indexed localToken,
        bytes32 indexed remoteToken,
        address indexed from,
        bytes32 to,
        uint256 amount,
        bytes extraData
    );

    //////////////////////////////////////////////////////////////
    ///                       Constants                        ///
    //////////////////////////////////////////////////////////////

    /// @notice The ERC-7528 standard address representing native ETH in token operations.
    address public constant ETH_ADDRESS = 0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE;

    //////////////////////////////////////////////////////////////
    ///                       Storage                          ///
    //////////////////////////////////////////////////////////////

    /// @notice Address of the CrossChainMessenger contract on this chain.
    ///
    /// @dev This messenger is responsible for sending cross-chain messages to the remote bridge.
    address public messenger;

    /// @notice Address/public key of the bridge contract on the remote chain.
    bytes32 public remoteBridge;

    /// @notice Mapping that stores deposit balances for token pairs between local and remote chains.
    ///
    /// @dev For standard ERC20 tokens and ETH, this tracks the total amount deposited and available for withdrawal.
    ///      CrossChainERC20 tokens bypass this mechanism as they use mint/burn instead.
    mapping(address localToken => mapping(bytes32 remoteToken => uint256 amount)) public deposits;

    //////////////////////////////////////////////////////////////
    ///                       Errors                           ///
    //////////////////////////////////////////////////////////////

    /// @notice Thrown when the ETH value sent with a transaction doesn't match the expected amount.
    error InvalidMsgValue();

    //////////////////////////////////////////////////////////////
    ///                       Modifiers                        ///
    //////////////////////////////////////////////////////////////

    /// @notice Restricts function access to Externally Owned Accounts (EOAs) only.
    ///
    /// @dev This modifier prevents smart contract wallets from accidentally depositing tokens,
    ///      as they may not be able to withdraw them on the remote chain. Note that this is not
    ///      completely safe against contracts calling code within their constructors, but provides
    ///      reasonable protection for typical use cases.
    modifier onlyEOA() {
        require(EOA.isSenderEOA(), "StandardBridge: function can only be called from an EOA");
        _;
    }

    /// @notice Restricts function access to the bridge contract on the remote chain.
    ///
    /// @dev Ensures that only legitimate cross-chain messages from the paired bridge can trigger
    ///      finalization functions. Validates both the immediate caller (messenger) and the
    ///      original cross-chain message sender.
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

    /// @notice Constructs the Bridge contract in an uninitialized state.
    ///
    /// @dev The constructor disables initializers to prevent the implementation contract from being initialized.
    ///      The actual initialization should be done through the initialize function after deployment.
    constructor() {
        _disableInitializers();
    }

    /// @notice Initializes the Bridge contract with the messenger and remote bridge addresses.
    ///
    /// @dev This function can only be called once due to the initializer modifier. It sets up the
    ///      cross-chain communication infrastructure required for bridge operations.
    ///
    /// @param messenger_    Address of the CrossChainMessenger contract on this chain.
    /// @param remoteBridge_ Address/public key of the bridge contract on the remote chain.
    function initialize(address messenger_, bytes32 remoteBridge_) external initializer {
        messenger = messenger_;
        remoteBridge = remoteBridge_;
    }

    /// @notice Bridges ERC20 tokens or ETH from this chain to a specified address on the remote chain.
    ///
    /// @dev This function handles the user-facing bridge initiation. For CrossChainERC20 tokens, it burns
    ///      the tokens on this chain. For standard tokens, it deposits them in the contract. For ETH,
    ///      it requires the exact amount to be sent as msg.value.
    ///
    /// @param localToken  Address of the ERC20 token on this chain (use ETH_ADDRESS for native ETH).
    /// @param remoteToken Address/public key of the corresponding token on the remote chain.
    /// @param to          Public key or address of the recipient on the remote chain (32 bytes for Solana
    /// compatibility).
    /// @param amount      Amount of tokens to bridge (must be uint64 for remote chain compatibility).
    /// @param extraData   Additional data to include with the bridge transaction for identification.
    function bridgeToken(address localToken, bytes32 remoteToken, bytes32 to, uint64 amount, bytes calldata extraData)
        public
        payable
        virtual
        onlyEOA
    {
        _initiateBridgeERC20(localToken, remoteToken, msg.sender, to, amount, extraData);
    }

    /// @notice Finalizes a token bridge transaction initiated from the remote chain.
    ///
    /// @dev This function can only be called by the remote bridge through the messenger system.
    ///      For CrossChainERC20 tokens, it mints new tokens. For standard tokens, it withdraws
    ///      from the deposit pool. Supports both ERC20 tokens and native ETH.
    ///
    /// @param localToken  Address of the ERC20 token on this chain (use ETH_ADDRESS for native ETH).
    /// @param remoteToken Address/public key of the corresponding token on the remote chain.
    /// @param from        Address/public key of the original sender on the remote chain.
    /// @param to          Address of the recipient on this chain.
    /// @param amount      Amount of tokens to transfer to the recipient.
    /// @param extraData   Additional data associated with the original bridge transaction.
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
                "Bridge: wrong remote token for CrossChain ERC20 local token"
            );

            localToken_.mint(to, amount);
        } else {
            deposits[localToken][remoteToken] = deposits[localToken][remoteToken] - amount;
            if (localToken == ETH_ADDRESS) {
                SafeTransferLib.safeTransferETH({to: to, amount: amount});
            } else {
                SafeTransferLib.safeTransfer({token: localToken, to: to, amount: amount});
            }
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

    /// @notice Internal function that handles the initiation of ERC20 token bridge transactions.
    ///
    /// @dev This function performs the actual token handling (burn for CrossChainERC20, deposit for others),
    ///      emits the bridge initiation event, and sends the cross-chain message to the remote bridge.
    ///      For ETH transfers, it validates that msg.value matches the amount parameter.
    ///
    /// @param localToken  Address of the ERC20 token on this chain.
    /// @param remoteToken Address/public key of the corresponding token on the remote chain.
    /// @param from        Address of the sender on this chain.
    /// @param to          Address/public key of the intended recipient on the remote chain.
    /// @param amount      Amount of tokens to bridge.
    /// @param extraData   Additional data to include with the bridge transaction.
    function _initiateBridgeERC20(
        address localToken,
        bytes32 remoteToken,
        address from,
        bytes32 to,
        uint64 amount,
        bytes memory extraData
    ) internal {
        if (_isCrossChainERC20(localToken)) {
            require(
                _isCorrectTokenPair(CrossChainERC20(localToken), remoteToken),
                "StandardBridge: wrong remote token for CrossChain ERC20 local token"
            );

            CrossChainERC20(localToken).burn(from, amount);
        } else {
            deposits[localToken][remoteToken] = deposits[localToken][remoteToken] + amount;
            if (localToken == ETH_ADDRESS) {
                if (msg.value != amount) {
                    revert InvalidMsgValue();
                }
            } else {
                SafeTransferLib.safeTransferFrom({token: localToken, from: from, to: address(this), amount: amount});
            }
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

    /// @notice Determines if a given token address implements the CrossChainERC20 interface.
    ///
    /// @dev Uses a static call to check if the token has a remoteToken() function that returns 32 bytes.
    ///      This is not a perfect check but provides reasonable confidence for the intended use case.
    ///
    /// @param token Address of the token to check.
    ///
    /// @return success True if the token appears to be a CrossChainERC20 token.
    function _isCrossChainERC20(address token) internal view returns (bool success) {
        (bool callSuccess, bytes memory data) = token.staticcall(abi.encodeCall(CrossChainERC20.remoteToken, ()));
        return callSuccess && data.length == 32;
    }

    /// @notice Validates that a CrossChainERC20 token is correctly paired with the specified remote token.
    ///
    /// @dev Checks that the CrossChainERC20's configured remote token matches the provided remote token address.
    ///
    /// @param localToken  The CrossChainERC20 token to validate.
    /// @param remoteToken The expected remote token address/public key.
    ///
    /// @return isCorrect True if the tokens are correctly paired.
    function _isCorrectTokenPair(CrossChainERC20 localToken, bytes32 remoteToken)
        internal
        view
        returns (bool isCorrect)
    {
        return localToken.remoteToken() == remoteToken;
    }
}
