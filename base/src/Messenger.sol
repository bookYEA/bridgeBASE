// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {SafeCall} from "optimism/packages/contracts-bedrock/src/libraries/SafeCall.sol";

/// @title Messenger
///
/// @notice The Messenger facilitates cross-chain communication between Base and Solana.
///         It allows users to send messages from Base to Solana and relay messages from Solana back to Base.
///         Messages are executed as Solana instructions on the destination chain.
contract Messenger {
    //////////////////////////////////////////////////////////////
    ///                       Events                           ///
    //////////////////////////////////////////////////////////////

    /// @notice Emitted whenever a message is successfully relayed and executed on this chain.
    ///
    /// @param messageHash Keccak256 hash of the message that was successfully relayed.
    event RelayedMessage(bytes32 indexed messageHash);

    /// @notice Emitted whenever a message fails to be relayed on this chain.
    ///
    /// @param messageHash Keccak256 hash of the message that failed to be relayed.
    event FailedRelayedMessage(bytes32 indexed messageHash);

    //////////////////////////////////////////////////////////////
    ///                       Errors                           ///
    //////////////////////////////////////////////////////////////

    /// @notice Thrown when the xChainMsgSender is not set.
    error XChainMsgSenderNotSet();

    /// @notice Thrown when the message is not already failed.
    error MessageNotAlreadyFailed();

    /// @notice Thrown when the message is already relayed.
    error MessageAlreadyRelayed();

    /// @notice Thrown when the message is already failed.
    error MessageAlreadyFailed();

    /// @notice Thrown when the message value is incorrect.
    error IncorrectMsgValue();

    /// @notice Thrown when the target is incorrect.
    error IncorrectTarget();

    /// @notice Thrown when the message failed to be relayed.
    error FailedToRelayMessage();

    //////////////////////////////////////////////////////////////
    ///                       Constants                        ///
    //////////////////////////////////////////////////////////////

    /// @notice Special address used as the tx.origin for gas estimation calls in the Messenger. This address
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

    //////////////////////////////////////////////////////////////
    ///                       Storage                          ///
    //////////////////////////////////////////////////////////////

    /// @notice Address of the messenger contract on the remote chain. Stored as bytes32 to handle non-EVM addresses
    ///         (like Solana) which may not fit into 20 bytes.
    bytes32 public immutable REMOTE_MESSENGER;

    /// @notice Mapping of message hashes to boolean receipt values indicating successful execution. A message will
    ///         only be present in this mapping if it has successfully been relayed on this chain, and therefore cannot
    ///         be relayed again.
    mapping(bytes32 messageHash => bool succeeded) public successfulMessages;

    /// @notice Mapping of message hashes to boolean values indicating failed execution attempts. A message will be
    ///         present in this mapping if and only if it has failed to execute at least once. Successfully executed
    ///         messages on first attempt won't appear here.
    mapping(bytes32 messageHash => bool failed) public failedMessages;

    /// @notice Address of the message sender that interacted with the messenger on the remote chain. If the value
    ///         equals DEFAULT_L2_SENDER, then no message is currently being executed. Use the xChainMsgSender() getter
    ///         which will revert if no message is active. Stored as bytes32 to handle non-EVM addresses which may not
    ///         fit into 20 bytes.
    bytes32 private _xChainMsgSender;

    //////////////////////////////////////////////////////////////
    ///                       Public Functions                 ///
    //////////////////////////////////////////////////////////////

    /// @notice Constructs the Messenger contract with immutable references.
    ///
    constructor(bytes32 remoteMessenger_) {
        REMOTE_MESSENGER = remoteMessenger_;
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
        require(_xChainMsgSender != DEFAULT_L2_SENDER, XChainMsgSenderNotSet());
        return _xChainMsgSender;
    }

    /// @notice Relays a message that was sent by the remote Messenger contract. Can only be executed via
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
            require(msg.value == value, IncorrectMsgValue());
            require(!failedMessages[messageHash], MessageAlreadyFailed());
        } else {
            require(msg.value == 0, IncorrectMsgValue());
            require(failedMessages[messageHash], MessageNotAlreadyFailed());
        }

        require(!_isUnsafeTarget(target), IncorrectTarget());
        require(!successfulMessages[messageHash], MessageAlreadyRelayed());

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
            require(tx.origin != ESTIMATION_ADDRESS, FailedToRelayMessage());
            return;
        }

        _xChainMsgSender = sender;
        bool success = SafeCall.call(target, gasleft() - RELAY_RESERVED_GAS, value, message);
        _xChainMsgSender = DEFAULT_L2_SENDER;

        if (success) {
            // This check is identical to one above, but it ensures that the same message cannot be relayed
            // twice, and adds a layer of protection against rentrancy.
            require(!successfulMessages[messageHash], MessageAlreadyRelayed());

            successfulMessages[messageHash] = true;
            emit RelayedMessage(messageHash);
        } else {
            failedMessages[messageHash] = true;
            emit FailedRelayedMessage(messageHash);

            // Revert in this case if the transaction was triggered by the estimation address. This should only be
            // possible during gas estimation or we have bigger problems. Reverting here will make the behavior of gas
            // estimation change such that the gas limit computed will be the amount required to relay the message, even
            // if that amount is greater than the minimum gas limit specified by the user.
            require(tx.origin != ESTIMATION_ADDRESS, FailedToRelayMessage());
        }
    }

    //////////////////////////////////////////////////////////////
    ///                       Internal Functions               ///
    //////////////////////////////////////////////////////////////

    /// @notice Checks whether the current message sender is the authorized remote messenger.
    ///
    /// @return True if the message is coming from the authorized remote messenger, false otherwise.
    function _isRemoteMessenger() internal view returns (bool) {
        return _bytes32ToAddress(REMOTE_MESSENGER) == msg.sender;
    }

    /// @notice Checks whether a given call target is a system address that could cause the messenger to perform an
    ///         unsafe action. This is NOT a mechanism for blocking user addresses. This is ONLY used to prevent
    ///         execution of messages to specific system addresses that could cause security issues (e.g., having the
    ///         Messenger send messages to itself).
    ///
    /// @param target Address of the contract to check for safety.
    ///
    /// @return True if the address is considered unsafe and should be blocked, false otherwise.
    function _isUnsafeTarget(address target) internal view returns (bool) {
        return target == address(this);
    }

    //////////////////////////////////////////////////////////////
    ///                       Private Functions                ///
    //////////////////////////////////////////////////////////////

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
