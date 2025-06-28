// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {LibClone} from "solady/utils/LibClone.sol";
import {ReentrancyGuardTransient} from "solady/utils/ReentrancyGuardTransient.sol";

import {Call, CallLib} from "./libraries/CallLib.sol";
import {MessageStorageLib} from "./libraries/MessageStorageLib.sol";
import {Pubkey} from "./libraries/SVMLib.sol";
import {TokenLib, Transfer} from "./libraries/TokenLib.sol";

import {Twin} from "./Twin.sol";

/// @title Portal
///
/// @notice The Portal enables sending calls from Solana to Base.
///
/// @dev Calls sent from Solana to Base are relayed via a Twin contract that is specific per Solana sender pubkey.
contract Bridge is ReentrancyGuardTransient {
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

    /// @notice Special pubkey used as the sender to represent the Solana bridge
    ///         Pubkey("111111111111111111111111111bridge")
    Pubkey public constant REMOTE_BRIDGE =
        Pubkey.wrap(0x0000000000000000000000000000000000000000000000000000000553ae31c3);

    /// @notice Address of the trusted relayer.
    address public immutable TRUSTED_RELAYER;

    /// @notice Address of the Twin beacon.
    address public immutable TWIN_BEACON;

    /// @notice Address of the Bridge Twin contract on Base.
    address public immutable BRIDGE_TWIN;

    // TODO: Better estimate.
    /// @notice Additional gas buffer reserved to ensure the correct execution of `relayMessagesEntrypoint`.
    uint64 private constant _RELAY_MESSAGES_ENTRYPOINT_GAS_BUFFER = 40_000;

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
    uint64 public lastNonce;

    //////////////////////////////////////////////////////////////
    ///                       Public Functions                 ///
    //////////////////////////////////////////////////////////////

    /// @notice Constructs the Portal contract with immutable references.
    ///
    constructor(address trustedRelayer, address twinBeacon, address bridgeTwin) {
        TRUSTED_RELAYER = trustedRelayer;
        TWIN_BEACON = twinBeacon;
        BRIDGE_TWIN = bridgeTwin;
    }

    /// @notice Sends a message to the Solana bridge.
    ///
    /// @param data The message data to be passed to the Solana bridge.
    function sendMessage(bytes calldata data) external {
        MessageStorageLib.sendMessage({sender: msg.sender, data: data});
    }

    /// @notice Relays messages sent from Solana to Base.
    ///
    /// @param messages The messages to relay.
    /// @param ismData Encoded ISM data used to verify the messages.
    function relayMessagesEntrypoint(IncomingMessage[] calldata messages, bytes calldata ismData)
        external
        nonReentrant
    {
        bool isTrustedRelayer = msg.sender == TRUSTED_RELAYER;
        if (isTrustedRelayer) {
            _ismVerify({messages: messages, ismData: ismData});
        }

        for (uint256 i; i < messages.length; i++) {
            IncomingMessage memory message = messages[i];
            this.validateAndExcute{gas: message.gasLimit}({message: message, isTrustedRelayer: isTrustedRelayer});
        }
    }

    /// @notice Validates and executes a message sent from Solana to Base.
    ///
    /// @dev This function can only be called by the entrypoint.
    ///
    /// @param message The message to relay.
    /// @param isTrustedRelayer Whether the caller was the trusted relayer.
    function validateAndExcute(IncomingMessage calldata message, bool isTrustedRelayer) external {
        // Check that the caller is the entrypoint.
        require(msg.sender == address(this), SenderIsNotEntrypoint());

        // NOTE: Intentionally not including the gas limit in the hash to allow for replays with higher gas limits.
        bytes32 messageHash = keccak256(abi.encode(message.nonce, message.sender, message.ty, message.data));

        // Check that the message has not already been relayed.
        require(!successes[messageHash], MessageAlreadySuccessfullyRelayed());

        // Check that the relay is allowed.
        if (isTrustedRelayer) {
            // TODO:Should the nonce be cached and only SSTOREd once?
            require(message.nonce == lastNonce + 1, NonceNotIncremental());
            lastNonce = message.nonce;

            require(!failures[messageHash], MessageAlreadyFailedToRelay());
        } else {
            require(failures[messageHash], MessageNotAlreadyFailedToRelay());
        }

        // Cover the gas cost upfront.
        failures[messageHash] = true;

        try this.relayMessage{gas: gasleft() - _RELAY_MESSAGES_ENTRYPOINT_GAS_BUFFER}({message: message}) {
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
    /// @dev This function can only be called by the entrypoint.
    ///
    /// @param message The message to relay.
    function relayMessage(IncomingMessage calldata message) external {
        // Check that the caller is the entrypoint.
        require(msg.sender == address(this), SenderIsNotEntrypoint());

        // // Special case where the message sneder is directly the Solana bridge.
        // // For now this is only the case when a Wrapped Token is deployed on Solana and is being registered on Base.
        // // When this happens the message is guaranteed to be a single operation that encode the parameters of the
        // // `registerRemoteToken` function.
        // if (message.sender == REMOTE_BRIDGE) {
        //     Operation memory operation = message.operations[0];
        //     (address localToken, Pubkey remoteToken, uint8 scalerExponent) =
        //         abi.decode(operation.data, (address, Pubkey, uint8));
        //     TokenLib.registerRemoteToken({
        //         localToken: localToken,
        //         remoteToken: remoteToken,
        //         scalerExponent: scalerExponent
        //     });
        //     return;
        // }

        // Get (and deploy if needed) the Twin contract.
        address twinAddress = twins[message.sender];
        if (twinAddress == address(0)) {
            twinAddress = LibClone.deployDeterministicERC1967BeaconProxy({
                beacon: TWIN_BEACON,
                salt: Pubkey.unwrap(message.sender)
            });
            twins[message.sender] = twinAddress;
        }

        if (message.ty == MessageType.Call) {
            Call memory call = abi.decode(message.data, (Call));
            Twin(payable(twins[message.sender])).execute{value: call.value}(call);
        } else if (message.ty == MessageType.Transfer) {
            Transfer memory transfer = abi.decode(message.data, (Transfer));
            TokenLib.finalizeTransfer(transfer);
        } else if (message.ty == MessageType.TransferAndCall) {
            (Transfer memory transfer, Call memory call) = abi.decode(message.data, (Transfer, Call));
            TokenLib.finalizeTransfer(transfer);
            Twin(payable(twins[message.sender])).execute{value: call.value}(call);
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
