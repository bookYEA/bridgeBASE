// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {Initializable} from "solady/utils/Initializable.sol";
import {LibClone} from "solady/utils/LibClone.sol";
import {ReentrancyGuardTransient} from "solady/utils/ReentrancyGuardTransient.sol";
import {UpgradeableBeacon} from "solady/utils/UpgradeableBeacon.sol";

import {Call} from "./libraries/CallLib.sol";
import {MessageStorageLib} from "./libraries/MessageStorageLib.sol";
import {SVMBridgeLib} from "./libraries/SVMBridgeLib.sol";
import {Ix, Pubkey} from "./libraries/SVMLib.sol";
import {SolanaTokenType, TokenLib, Transfer} from "./libraries/TokenLib.sol";

import {Twin} from "./Twin.sol";

/// @title Bridge
///
/// @notice The Bridge enables sending calls from Solana to Base.
///
/// @dev Calls sent from Solana to Base are relayed via a Twin contract that is specific per Solana sender pubkey.
contract Bridge is ReentrancyGuardTransient, Initializable {
    //////////////////////////////////////////////////////////////
    ///                       Events                           ///
    //////////////////////////////////////////////////////////////

    /// @notice Emitted whenever a message is successfully relayed and executed.
    ///
    /// @param messageHash Keccak256 hash of the message that was successfully relayed.
    event MessageSuccessfullyRelayed(bytes32 indexed messageHash);

    /// @notice Emitted whenever a message fails to be relayed.
    ///
    /// @param messageHash Keccak256 hash of the message that failed to be relayed.
    event FailedToRelayMessage(bytes32 indexed messageHash);

    //////////////////////////////////////////////////////////////
    ///                       Errors                           ///
    //////////////////////////////////////////////////////////////

    /// @notice Thrown when the ISM verification fails.
    error ISMVerificationFailed();

    /// @notice Thrown when doing gas estimation and the call's gas left is insufficient to cover the `minGas` plus the
    ///         `reservedGas`.
    error EstimationInsufficientGas();

    /// @notice Thrown when the call execution fails.
    error ExecutionFailed();

    /// @notice Thrown when the sender is not the entrypoint.
    error SenderIsNotEntrypoint();

    /// @notice Thrown when the nonce is not incremental.
    error NonceNotIncremental();

    /// @notice Thrown when a message has already been successfully relayed.
    error MessageAlreadySuccessfullyRelayed();

    /// @notice Thrown when a message has already failed to relay.
    error MessageAlreadyFailedToRelay();

    /// @notice Thrown when a message has not been marked as failed by the relayer but a user tries to relay it
    /// manually.
    error MessageNotAlreadyFailedToRelay();

    /// @notice Thrown when an Anchor instruction is invalid.
    error UnsafeIxTarget();

    //////////////////////////////////////////////////////////////
    ///                       Structs                          ///
    //////////////////////////////////////////////////////////////

    /// @notice Enum containing operation types.
    enum MessageType {
        Call,
        Transfer,
        TransferAndCall
    }

    /// @notice Message sent from Solana to Base.
    ///
    /// @custom:field nonce Unique nonce for the message.
    /// @custom:field sender The Solana sender's pubkey.
    /// @custom:field gasLimit The gas limit for the message execution.
    /// @custom:field operations The operations to be executed.
    struct IncomingMessage {
        uint64 nonce;
        Pubkey sender;
        uint64 gasLimit;
        MessageType ty;
        bytes data;
    }

    //////////////////////////////////////////////////////////////
    ///                       Constants                        ///
    //////////////////////////////////////////////////////////////

    /// @notice Special address used as the tx origin for gas estimation calls.
    ///
    /// @dev You only need to use this address if the minimum gas limit specified by the user is not actually enough to
    ///      execute the given message and you're attempting to estimate the actual necessary gas limit. We use
    ///      address(1) because it's the ecrecover precompile and therefore guaranteed to never have any code on any EVM
    ///      chain.
    address public constant ESTIMATION_ADDRESS = address(1);

    /// @notice Pubkey of the remote bridge on Solana.
    Pubkey public immutable REMOTE_BRIDGE;

    /// @notice Address of the trusted relayer.
    address public immutable TRUSTED_RELAYER;

    // TODO: Better estimate.
    /// @notice Additional gas buffer reserved to ensure the correct execution of `relayMessages`.
    uint64 private constant _RELAY_MESSAGES_GAS_BUFFER = 40_000;

    //////////////////////////////////////////////////////////////
    ///                       Storage                          ///
    //////////////////////////////////////////////////////////////

    /// @notice Mapping of message hashes to boolean values indicating successful execution. A message will only be
    ///         present in this mapping if it has successfully been executed, and therefore cannot be executed again.
    mapping(bytes32 messageHash => bool success) public successes;

    /// @notice Mapping of message hashes to boolean values indicating failed execution attempts. A message will be
    ///         present in this mapping if and only if it has failed to execute at least once. Successfully executed
    ///         messages on first attempt won't appear here.
    mapping(bytes32 messageHash => bool failure) public failures;

    /// @notice Mapping of Solana owner pubkeys to their Twin contract addresses.
    mapping(Pubkey owner => address twinAddress) public twins;

    /// @notice The last message's nonce relayed by the trusted relayer.
    uint64 public nextIncomingNonce;

    /// @notice Address of the Twin beacon.
    address public twinBeacon;

    //////////////////////////////////////////////////////////////
    ///                       Public Functions                 ///
    //////////////////////////////////////////////////////////////

    /// @notice Constructs the Bridge contract.
    ///
    /// @param remoteBridge The pubkey of the remote bridge on Solana.
    /// @param trustedRelayer The address of the trusted relayer.
    constructor(Pubkey remoteBridge, address trustedRelayer) {
        REMOTE_BRIDGE = remoteBridge;
        TRUSTED_RELAYER = trustedRelayer;
        _disableInitializers();
    }

    /// @param initialOwner The initial owner of the Twin beacon proxy.
    function initialize(address initialOwner) external reinitializer(1) {
        address twinImpl = address(new Twin());
        twinBeacon = address(new UpgradeableBeacon(initialOwner, twinImpl));
    }

    /// @notice Get the current root of the MMR.
    ///
    /// @return The current root of the MMR.
    function getRoot() external view returns (bytes32) {
        return MessageStorageLib.getMessageStorageLibStorage().root;
    }

    /// @notice Get the last outgoing Message nonce.
    ///
    /// @return The last outgoing Message nonce.
    function getLastOutgoingNonce() external view returns (uint64) {
        return MessageStorageLib.getMessageStorageLibStorage().lastOutgoingNonce;
    }

    /// @notice Generates a Merkle proof for a specific leaf in the MMR.
    ///
    /// @dev This function may consume significant gas for large MMRs (O(log N) storage reads).
    ///
    /// @param leafIndex The 0-indexed position of the leaf to prove.
    ///
    /// @return proof Array of sibling hashes for the proof.
    /// @return totalLeafCount The total number of leaves when proof was generated.
    function generateProof(uint64 leafIndex) external view returns (bytes32[] memory proof, uint64 totalLeafCount) {
        return MessageStorageLib.generateProof(leafIndex);
    }

    /// @notice Bridges a call to the Solana bridge.
    ///
    /// @param ixs The Solana instructions.
    function bridgeCall(Ix[] memory ixs) external {
        MessageStorageLib.sendMessage({sender: msg.sender, data: SVMBridgeLib.serializeCall(ixs)});
    }

    /// @notice Bridges a transfer with optional an optional list of instructions to the Solana bridge.
    ///
    /// @param transfer The token transfer to execute.
    /// @param ixs The optional Solana instructions.
    function bridgeToken(Transfer calldata transfer, Ix[] memory ixs) external payable {
        SolanaTokenType transferType = TokenLib.initializeTransfer({transfer: transfer});
        MessageStorageLib.sendMessage({
            sender: msg.sender,
            data: SVMBridgeLib.serializeTransfer({transfer: transfer, tokenType: transferType, ixs: ixs})
        });
    }

    /// @notice Relays messages sent from Solana to Base.
    ///
    /// @param messages The messages to relay.
    /// @param ismData Encoded ISM data used to verify the messages.
    function relayMessages(IncomingMessage[] calldata messages, bytes calldata ismData) external nonReentrant {
        bool isTrustedRelayer = msg.sender == TRUSTED_RELAYER;
        if (isTrustedRelayer) {
            _ismVerify({messages: messages, ismData: ismData});
        }

        for (uint256 i; i < messages.length; i++) {
            IncomingMessage memory message = messages[i];
            this.__validateAndRelay{gas: message.gasLimit}({message: message, isTrustedRelayer: isTrustedRelayer});
        }
    }

    /// @notice Validates and relays a message sent from Solana to Base.
    ///
    /// @dev This function can only be called from `relayMessages`.
    ///
    /// @param message The message to relay.
    /// @param isTrustedRelayer Whether the caller was the trusted relayer.
    function __validateAndRelay(IncomingMessage calldata message, bool isTrustedRelayer) external {
        _assertSenderIsEntrypoint();

        // NOTE: Intentionally not including the gas limit in the hash to allow for replays with higher gas limits.
        bytes32 messageHash = keccak256(abi.encode(message.nonce, message.sender, message.ty, message.data));

        // Check that the message has not already been relayed.
        require(!successes[messageHash], MessageAlreadySuccessfullyRelayed());

        // Check that the relay is allowed.
        if (isTrustedRelayer) {
            // TODO:Should the nonce be cached and only SSTOREd once?
            require(message.nonce == nextIncomingNonce, NonceNotIncremental());
            nextIncomingNonce = message.nonce + 1;

            require(!failures[messageHash], MessageAlreadyFailedToRelay());
        } else {
            require(failures[messageHash], MessageNotAlreadyFailedToRelay());
        }

        // Cover the gas cost upfront.
        failures[messageHash] = true;

        // Use the message's specified gas limit for execution, ensuring it doesn't exceed available gas
        uint256 gasLeft = gasleft();
        uint256 gasForRelay = gasLeft > _RELAY_MESSAGES_GAS_BUFFER
            ? (
                gasLeft - _RELAY_MESSAGES_GAS_BUFFER > message.gasLimit
                    ? message.gasLimit
                    : gasLeft - _RELAY_MESSAGES_GAS_BUFFER
            )
            : 0;

        try this.__relayMessage{gas: gasForRelay}({message: message}) {
            // Register the call as successful.
            delete failures[messageHash];
            successes[messageHash] = true;

            emit MessageSuccessfullyRelayed(messageHash);
        } catch {
            emit FailedToRelayMessage(messageHash);

            // Revert for gas estimation.
            if (tx.origin == ESTIMATION_ADDRESS) {
                revert ExecutionFailed();
            }
        }
    }

    /// @notice Relays a message sent from Solana to Base.
    ///
    /// @dev This function can only be called from `__validateAndRelay`.
    ///
    /// @param message The message to relay.
    function __relayMessage(IncomingMessage calldata message) external {
        _assertSenderIsEntrypoint();

        // Special case where the message sender is directly the Solana bridge.
        // For now this is only the case when a Wrapped Token is deployed on Solana and is being registered on Base.
        // When this happens the message is guaranteed to be a single operation that encode the parameters of the
        // `registerRemoteToken` function.
        if (message.sender == REMOTE_BRIDGE) {
            (address localToken, Pubkey remoteToken, uint8 scalerExponent) =
                abi.decode(message.data, (address, Pubkey, uint8));

            TokenLib.registerRemoteToken({
                localToken: localToken,
                remoteToken: remoteToken,
                scalerExponent: scalerExponent
            });
            return;
        }

        // Get (and deploy if needed) the Twin contract.
        address twinAddress = twins[message.sender];
        if (twinAddress == address(0)) {
            twinAddress = LibClone.deployDeterministicERC1967BeaconProxy({
                beacon: twinBeacon,
                salt: Pubkey.unwrap(message.sender)
            });
            twins[message.sender] = twinAddress;
        }

        if (message.ty == MessageType.Call) {
            Call memory call = abi.decode(message.data, (Call));
            Twin(payable(twins[message.sender])).execute(call);
        } else if (message.ty == MessageType.Transfer) {
            Transfer memory transfer = abi.decode(message.data, (Transfer));
            TokenLib.finalizeTransfer(transfer);
        } else if (message.ty == MessageType.TransferAndCall) {
            (Transfer memory transfer, Call memory call) = abi.decode(message.data, (Transfer, Call));
            TokenLib.finalizeTransfer(transfer);
            Twin(payable(twins[message.sender])).execute(call);
        }
    }

    //////////////////////////////////////////////////////////////
    ///                       Internal Functions                ///
    //////////////////////////////////////////////////////////////

    /// @inheritdoc ReentrancyGuardTransient
    ///
    /// @dev We know Base mainnet supports transient storage.
    function _useTransientReentrancyGuardOnlyOnMainnet() internal pure override returns (bool) {
        return false;
    }

    //////////////////////////////////////////////////////////////
    ///                       Private Functions                ///
    //////////////////////////////////////////////////////////////

    /// @notice Asserts that the caller is the entrypoint.
    function _assertSenderIsEntrypoint() private view {
        require(msg.sender == address(this), SenderIsNotEntrypoint());
    }

    /// @notice Checks whether the ISM verification is successful.
    ///
    /// @param messages The messages to verify.
    /// @param ismData Encoded ISM data used to verify the call.
    function _ismVerify(IncomingMessage[] calldata messages, bytes calldata ismData) private pure {
        messages; // Silence unused variable warning.
        ismData; // Silence unused variable warning.

        // TODO: Plug some ISM verification here.
        require(true, ISMVerificationFailed());
    }
}
