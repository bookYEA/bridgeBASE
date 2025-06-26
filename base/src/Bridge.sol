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

    /// @notice Enum containing message types.
    enum MessageType {
        Transfer,
        Call
    }

    /// @notice Message sent from Solana to Base.
    ///
    /// @custom:field nonce Unique nonce for the message.
    /// @custom:field sender The Solana sender's pubkey.
    /// @custom:field gasLimit The gas limit for the message execution.
    /// @custom:field msgType The type of the message.
    /// @custom:field data The abi encoded data for the message.
    ///                    Transfer => abi.encode(Transfer)
    ///                    Call => abi.encode(Call)
    struct Message {
        uint64 nonce;
        Pubkey sender;
        uint64 gasLimit;
        MessageType messageType;
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

    /// @notice Address of the trusted relayer.
    address public immutable TRUSTED_RELAYER;

    /// @notice Address of the Twin beacon.
    address public immutable TWIN_BEACON;

    /// @notice Additional gas buffer reserved to ensure the correct execution of `relayMessagesEntrypoint`.
    uint64 private constant _RELAY_MESSAGES_ENTRYPOINT_GAS_BUFFER = 40_000;

    /// @notice Gas reserved for the dynamic parts of the `CALL` opcode.
    uint64 private constant _CALL_OVERHEAD_GAS = 40_000;

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
    constructor(address trustedRelayer, address twinBeacon) {
        TRUSTED_RELAYER = trustedRelayer;
        TWIN_BEACON = twinBeacon;
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
    function relayMessagesEntrypoint(Message[] calldata messages, bytes calldata ismData) external nonReentrant {
        bool isTrustedRelayer = msg.sender == TRUSTED_RELAYER;
        if (isTrustedRelayer) {
            _ismVerify({messages: messages, ismData: ismData});
        }

        for (uint256 i; i < messages.length; i++) {
            Message memory message = messages[i];

            // NOTE: Intentionally not including the gas limit in the hash to allow for replays with higher gas limits.
            bytes32 messageHash =
                keccak256(abi.encode(message.nonce, message.sender, message.messageType, message.data));

            // Ensures sufficient gas for execution and cleanup.
            uint256 reservedGas = _CALL_OVERHEAD_GAS + _RELAY_MESSAGES_ENTRYPOINT_GAS_BUFFER;
            if (message.gasLimit < reservedGas) {
                failures[messageHash] = true;
                emit FailedToRelayMessage(messageHash);

                // Revert for gas estimation.
                if (tx.origin == ESTIMATION_ADDRESS) {
                    revert EstimationInsufficientGas();
                }

                return;
            }

            try this.relayMessage{gas: message.gasLimit - _RELAY_MESSAGES_ENTRYPOINT_GAS_BUFFER}({
                message: message,
                messageHash: messageHash,
                isTrustedRelayer: isTrustedRelayer
            }) {
                // Register the call as successful.
                successes[messageHash] = true;
                if (failures[messageHash]) {
                    delete failures[messageHash];
                }

                emit MessageSuccessfullyRelayed(messageHash);
            } catch (bytes memory reason) {
                // Some mandatory invariant has been violated, we should revert.
                if (bytes4(reason) != ExecutionFailed.selector) {
                    assembly {
                        revert(add(reason, 32), mload(reason))
                    }
                }

                // Otherwise the user call itself reverted, register the call as failed.
                failures[messageHash] = true;
                emit FailedToRelayMessage(messageHash);

                // Revert for gas estimation.
                if (tx.origin == ESTIMATION_ADDRESS) {
                    revert ExecutionFailed();
                }

                return;
            }
        }
    }

    /// @notice Relays a message sent from Solana to Base.
    ///
    /// @dev This function can only be called by the entrypoint.
    ///
    /// @param message The message to relay.
    /// @param messageHash The hash of the message.
    /// @param isTrustedRelayer Whether the caller was the trusted relayer.
    function relayMessage(Message calldata message, bytes32 messageHash, bool isTrustedRelayer) external {
        // Check that the caller is the entrypoint.
        require(msg.sender == address(this), SenderIsNotEntrypoint());

        // Check that the message has not already been relayed.
        require(!successes[messageHash], MessageAlreadySuccessfullyRelayed());

        // Check that the relay is allowed.
        if (isTrustedRelayer) {
            require(message.nonce == lastNonce + 1, NonceNotIncremental());
            lastNonce = message.nonce;

            require(!failures[messageHash], MessageAlreadyFailedToRelay());
        } else {
            require(failures[messageHash], MessageNotAlreadyFailedToRelay());
        }

        // Get (and deploy if needed) the Twin contract.
        address twinAddress = twins[message.sender];
        if (twinAddress == address(0)) {
            twinAddress = LibClone.deployDeterministicERC1967BeaconProxy({
                beacon: TWIN_BEACON,
                salt: Pubkey.unwrap(message.sender)
            });
            twins[message.sender] = twinAddress;
        }

        if (message.messageType == MessageType.Transfer) {
            Transfer memory transfer = abi.decode(message.data, (Transfer));

            // TODO: Do I need to wrap the token transfer in a try/catch with ExecutionFailed?
            TokenLib.finalizeTransfer(transfer);
        } else if (message.messageType == MessageType.Call) {
            Call memory call = abi.decode(message.data, (Call));
            try Twin(payable(twinAddress)).execute{value: call.value}(call) {}
            catch {
                revert ExecutionFailed();
            }
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
    function _ismVerify(Message[] calldata messages, bytes calldata ismData) private pure {
        messages; // Silence unused variable warning.
        ismData; // Silence unused variable warning.

        // TODO: Plug some ISM verification here.
        require(true, ISMVerificationFailed());
    }
}
