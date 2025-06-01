// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {Encoding} from "optimism/packages/contracts-bedrock/src/libraries/Encoding.sol";
import {SafeCall} from "optimism/packages/contracts-bedrock/src/libraries/SafeCall.sol";
import {Initializable} from "solady/utils/Initializable.sol";

import {MessagePasser} from "./MessagePasser.sol";
import {Encoder} from "./libraries/Encoder.sol";

/// @title CrossChainMessenger
///
/// @notice The CrossChainMessenger facilitates cross-chain communication between Base and Solana.
///         It allows users to send messages from Base to Solana and relay messages from Solana back to Base.
///         Messages are executed as Solana instructions on the destination chain.
contract CrossChainMessenger is Initializable {
    //////////////////////////////////////////////////////////////
    ///                       Structs                          ///
    //////////////////////////////////////////////////////////////

    /// @notice Struct representing a messenger payload for cross-chain communication.
    ///
    /// @custom:field nonce  Unique nonce of the message to prevent replay attacks.
    /// @custom:field sender Address of the message sender on the origin chain.
    /// @custom:field ixs    Array of Solana instructions to execute on the destination chain.
    struct MessengerPayload {
        uint256 nonce;
        address sender;
        MessagePasser.Instruction[] ixs;
    }

    //////////////////////////////////////////////////////////////
    ///                       Events                           ///
    //////////////////////////////////////////////////////////////

    /// @notice Emitted whenever a message is sent to the remote chain.
    ///
    /// @param sender       Address of the sender of the message on this chain.
    /// @param ixs          Array of Solana instructions to be executed on the remote chain.
    /// @param messageNonce Unique nonce attached to the message for identification and replay protection.
    event SentMessage(address indexed sender, MessagePasser.Instruction[] ixs, uint256 messageNonce);

    /// @notice Emitted whenever a message is successfully relayed and executed on this chain.
    ///
    /// @param messageHash Keccak256 hash of the message that was successfully relayed.
    event RelayedMessage(bytes32 indexed messageHash);

    /// @notice Emitted whenever a message fails to be relayed on this chain.
    ///
    /// @param messageHash Keccak256 hash of the message that failed to be relayed.
    event FailedRelayedMessage(bytes32 indexed messageHash);

    //////////////////////////////////////////////////////////////
    ///                       Constants                        ///
    //////////////////////////////////////////////////////////////

    /// @notice Current message version identifier used for encoding message nonces. Allows for future message format
    ///         upgrades while maintaining backward compatibility.
    uint16 public constant MESSAGE_VERSION = 1;

    /// @notice Special address used as the tx.origin for gas estimation calls in the CrossChainMessenger. This address
    ///         should only be used during gas estimation to determine the actual necessary gas limit when the
    ///         user-specified minimum gas limit is insufficient. We use address(1) because it's the ecrecover
    ///         precompile and therefore guaranteed to never have code on any EVM chain.
    address internal constant ESTIMATION_ADDRESS = address(1);

    /// @notice Gas reserved for finalizing the execution of `relayMessage` after the safe call. This ensures there's
    ///         enough gas to complete the relay process after the target call.
    uint64 public constant RELAY_RESERVED_GAS = 40_000;

    /// @notice Gas buffer reserved for execution between the `hasMinGas` check and the external call in `relayMessage`.
    ///         This accounts for gas consumed in the relay setup before the actual call.
    uint64 public constant RELAY_GAS_CHECK_BUFFER = 5_000;

    /// @notice Default value for cross-chain message sender when no message is being executed. This value is non-zero
    ///         to reduce gas costs of message passing transactions by avoiding zero-to-non-zero storage writes.
    bytes32 internal constant DEFAULT_L2_SENDER =
        bytes32(0x000000000000000000000000000000000000000000000000000000000000dEaD);

    /// @notice Address of the MessagePasser contract on this chain that handles cross-chain message initiation.
    address public immutable SOLANA_MESSAGE_PASSER;

    /// @notice Solana program ID of the Solana Messenger program that will process messages on Solana. Stored as
    ///         bytes32 to accommodate Solana's 32-byte address format.
    bytes32 public immutable SOLANA_MESSENGER_PROGRAM;

    //////////////////////////////////////////////////////////////
    ///                       Storage                          ///
    //////////////////////////////////////////////////////////////

    /// @notice Address of the messenger contract on the remote chain. Stored as bytes32 to handle non-EVM addresses
    ///         (like Solana) which may not fit into 20 bytes.
    bytes32 public remoteMessenger;

    /// @notice Mapping of message hashes to boolean receipt values indicating successful execution. A message will
    ///         only be present in this mapping if it has successfully been relayed on this chain, and therefore cannot
    ///         be relayed again.
    mapping(bytes32 messageHash => bool succeeded) public successfulMessages;

    /// @notice Mapping of message hashes to boolean values indicating failed execution attempts. A message will be
    ///         present in this mapping if and only if it has failed to execute at least once. Successfully executed
    ///         messages on first attempt won't appear here.
    mapping(bytes32 messageHash => bool failed) public failedMessages;

    /// @notice Nonce for the next message to be sent, without the message version applied. Use the messageNonce()
    ///         getter which applies the message version to get the actual nonce used for the message.
    uint240 internal _msgNonce;

    /// @notice Address of the message sender that interacted with the messenger on the remote chain. If the value
    ///         equals DEFAULT_L2_SENDER, then no message is currently being executed. Use the xChainMsgSender() getter
    ///         which will revert if no message is active. Stored as bytes32 to handle non-EVM addresses which may not
    ///         fit into 20 bytes.
    bytes32 private _xChainMsgSender;

    //////////////////////////////////////////////////////////////
    ///                       Public Functions                 ///
    //////////////////////////////////////////////////////////////

    /// @notice Constructs the CrossChainMessenger contract with immutable references.
    ///
    /// @param solanaMessagePasser_    Address of the MessagePasser contract on this chain.
    /// @param solanaMessengerProgram_ Solana program ID of the messenger program on Solana.
    constructor(address solanaMessagePasser_, bytes32 solanaMessengerProgram_) {
        SOLANA_MESSAGE_PASSER = solanaMessagePasser_;
        SOLANA_MESSENGER_PROGRAM = solanaMessengerProgram_;
        _disableInitializers();
    }

    /// @notice Retrieves the address of the sender that initiated the message on the remote chain. This function can
    ///         only be called during the execution of a cross-chain message.
    ///
    /// @dev Will revert if there is no message currently being executed (i.e., when called outside of the context of
    ///      relayMessage execution).
    ///
    /// @return The address of the message sender from the remote chain, formatted as bytes32 to accommodate non-EVM
    ///         address formats.
    function xChainMsgSender() external view returns (bytes32) {
        require(_xChainMsgSender != DEFAULT_L2_SENDER, "CrossChainMessenger: xChainMsgSender is not set");
        return _xChainMsgSender;
    }

    /// @notice Retrieves the next message nonce with version encoding applied. The message version is encoded in the
    ///         upper bytes of the nonce, allowing for different message structures to be supported in future versions.
    ///
    /// @return The nonce that will be assigned to the next message sent, with message version encoded.
    function messageNonce() public view returns (uint256) {
        return Encoding.encodeVersionedNonce(_msgNonce, MESSAGE_VERSION);
    }

    /// @notice Initializes the CrossChainMessenger with the remote messenger address. This function can only be called
    ///         once due to the initializer modifier.
    ///
    /// @dev The xChainMsgSender is only set to default if it's uninitialized (fresh deployment). This prevents
    ///      resetting during upgrades, which could enable reentrant message execution and allow malicious actors to
    ///      replay messages.
    ///
    /// @param remoteMessenger_ Address of the messenger contract on the remote chain.
    function initialize(bytes32 remoteMessenger_) external initializer {
        // We only want to set the xChainMsgSender to the default value if it hasn't been initialized yet, meaning that
        // this is a fresh contract deployment. This prevents resetting the xChainMsgSender to the default value during
        // an upgrade, which would enable reentrant message execution to sandwich the upgrade and replay a message
        // twice.
        if (_xChainMsgSender == 0) {
            _xChainMsgSender = DEFAULT_L2_SENDER;
        }

        remoteMessenger = remoteMessenger_;
    }

    /// @notice Sends a message containing Solana instructions to be executed on the remote chain. The message will be
    ///         wrapped in a MessengerPayload and passed through the MessagePasser.
    ///
    /// @dev If the call on the destination chain always reverts, the message will be unrelayable and any ETH sent will
    ///      be permanently locked. The same occurs if the target on the remote chain is considered unsafe.
    ///
    /// @param messageIxs Array of Solana instructions to execute on the destination chain.
    function sendMessage(MessagePasser.Instruction[] calldata messageIxs) external {
        uint256 nonce = messageNonce();

        MessagePasser.Instruction[] memory ixs = new MessagePasser.Instruction[](1);
        ixs[0] = MessagePasser.Instruction({
            programId: SOLANA_MESSENGER_PROGRAM,
            accounts: new MessagePasser.AccountMeta[](0),
            data: Encoder.encodeMessengerPayload(MessengerPayload({nonce: nonce, sender: msg.sender, ixs: messageIxs}))
        });

        // Triggers a message to the remote messenger. Note that the amount of gas provided to the
        // message is the amount of gas requested by the user PLUS the base gas value. We want to
        // guarantee the property that the call to the target contract will always have at least
        // the minimum gas limit specified by the user.
        _sendMessage(ixs);

        emit SentMessage(msg.sender, messageIxs, nonce);

        unchecked {
            ++_msgNonce;
        }
    }

    /// @notice Relays a message that was sent by the remote CrossChainMessenger contract. Can only be executed via
    ///         cross-chain call from the remote messenger OR if the message previously failed and is being replayed.
    ///
    /// @dev Gas estimation: If the transaction origin is ESTIMATION_ADDRESS, failures will cause reverts to help
    ///      compute accurate gas limits during estimation.
    ///
    /// @param nonce       Unique nonce of the message being relayed.
    /// @param sender      Address of the user who sent the message on the remote chain.
    /// @param target      Address that the message is targeted at on this chain.
    /// @param value       ETH value to send with the message execution.
    /// @param minGasLimit Minimum amount of gas that the message must be executed with.
    /// @param message     Encoded message data to send to the target address.
    function relayMessage(
        uint256 nonce,
        bytes32 sender,
        address target,
        uint256 value,
        uint256 minGasLimit,
        bytes calldata message
    ) external payable {
        bytes32 messageHash =
            keccak256(abi.encodeCall(this.relayMessage, (nonce, sender, target, value, minGasLimit, message)));

        if (_isRemoteMessenger()) {
            require(msg.value == value, "CrossChainMessenger: value must be equal to the value sent");
            require(!failedMessages[messageHash], "CrossChainMessenger: message cannot be replayed");
        } else {
            require(msg.value == 0, "CrossChainMessenger: value must be zero unless message is from a system address");
            require(failedMessages[messageHash], "CrossChainMessenger: message cannot be replayed");
        }

        require(!_isUnsafeTarget(target), "CrossChainMessenger: cannot send message to blocked system address");
        require(!successfulMessages[messageHash], "CrossChainMessenger: message has already been relayed");

        // If there is not enough gas left to perform the external call and finish the execution, return early and
        // assign the message to the failedMessages mapping.
        //
        // We are asserting that we have enough gas to:
        // 1. Call the target contract (_minGasLimit + RELAY_CALL_OVERHEAD + RELAY_GAS_CHECK_BUFFER)
        //   1.a. The RELAY_CALL_OVERHEAD is included in `hasMinGas`.
        // 2. Finish the execution after the external call (RELAY_RESERVED_GAS).
        //
        // If `_xChainMsgSender` is not the default sender, this function is being re-entered. This marks the message
        // as failed to allow it to be replayed.
        if (
            !SafeCall.hasMinGas(minGasLimit, RELAY_RESERVED_GAS + RELAY_GAS_CHECK_BUFFER)
                || _xChainMsgSender != DEFAULT_L2_SENDER
        ) {
            failedMessages[messageHash] = true;
            emit FailedRelayedMessage(messageHash);

            // Revert in this case if the transaction was triggered by the estimation address. This should only be
            // possible during gas estimation or we have bigger problems. Reverting here will make the behavior of gas
            // estimation change such that the gas limit computed will be the amount required to relay the message, even
            // if that amount is greater than the minimum gas limit specified by the user.
            require(tx.origin != ESTIMATION_ADDRESS, "CrossChainMessenger: failed to relay message");
            return;
        }

        _xChainMsgSender = sender;
        bool success = SafeCall.call(target, gasleft() - RELAY_RESERVED_GAS, value, message);
        _xChainMsgSender = DEFAULT_L2_SENDER;

        if (success) {
            // This check is identical to one above, but it ensures that the same message cannot be relayed
            // twice, and adds a layer of protection against rentrancy.
            require(!successfulMessages[messageHash], "CrossChainMessenger: message has already been relayed");

            successfulMessages[messageHash] = true;
            emit RelayedMessage(messageHash);
        } else {
            failedMessages[messageHash] = true;
            emit FailedRelayedMessage(messageHash);

            // Revert in this case if the transaction was triggered by the estimation address. This should only be
            // possible during gas estimation or we have bigger problems. Reverting here will make the behavior of gas
            // estimation change such that the gas limit computed will be the amount required to relay the message, even
            // if that amount is greater than the minimum gas limit specified by the user.
            require(tx.origin != ESTIMATION_ADDRESS, "CrossChainMessenger: failed to relay message");
        }
    }

    //////////////////////////////////////////////////////////////
    ///                       Internal Functions               ///
    //////////////////////////////////////////////////////////////

    /// @notice Sends a low-level message to the remote chain via the MessagePasser contract.
    ///
    /// @param ixs Array of Solana instructions to be passed to the MessagePasser for cross-chain execution.
    function _sendMessage(MessagePasser.Instruction[] memory ixs) internal {
        MessagePasser(SOLANA_MESSAGE_PASSER).initiateWithdrawal(ixs);
    }

    /// @notice Checks whether the current message sender is the authorized remote messenger. Virtual function to allow
    ///         for different verification logic in derived contracts.
    ///
    /// @return True if the message is coming from the authorized remote messenger, false otherwise.
    function _isRemoteMessenger() internal view virtual returns (bool) {
        return _bytes32ToAddress(remoteMessenger) == msg.sender;
    }

    /// @notice Checks whether a given call target is a system address that could cause the messenger to perform an
    ///         unsafe action. This is NOT a mechanism for blocking user addresses. This is ONLY used to prevent
    ///         execution of messages to specific system addresses that could cause security issues (e.g., having the
    ///         CrossChainMessenger send messages to itself).
    ///
    /// @param target Address of the contract to check for safety.
    ///
    /// @return True if the address is considered unsafe and should be blocked, false otherwise.
    function _isUnsafeTarget(address target) internal view virtual returns (bool) {
        return target == address(this);
    }

    //////////////////////////////////////////////////////////////
    ///                       Private Functions                ///
    //////////////////////////////////////////////////////////////

    /// @notice Returns the maximum of two uint256 values.
    ///
    /// @param a First value to compare.
    /// @param b Second value to compare.
    function _max(uint256 a, uint256 b) private pure returns (uint256) {
        return a > b ? a : b;
    }

    /// @notice Converts a bytes32 value to an address by truncating to the last 20 bytes. Used to convert cross-chain
    ///         addresses stored as bytes32 back to Ethereum addresses.
    ///
    /// @param value The bytes32 value to convert to an address.
    ///
    /// @return The extracted address from the bytes32 value.
    function _bytes32ToAddress(bytes32 value) private pure returns (address) {
        return address(uint160(uint256(value)));
    }
}
