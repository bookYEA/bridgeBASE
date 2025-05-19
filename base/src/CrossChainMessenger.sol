// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {ISolanaMessagePasser} from "./interfaces/ISolanaMessagePasser.sol";
import {Encoding} from "optimism/packages/contracts-bedrock/src/libraries/Encoding.sol";
import {SafeCall} from "optimism/packages/contracts-bedrock/src/libraries/SafeCall.sol";
import {Initializable} from "solady/utils/Initializable.sol";

contract CrossChainMessenger is Initializable {
    struct MessengerPayload {
        uint256 nonce;
        address sender;
        bytes message;
    }

    //////////////////////////////////////////////////////////////
    ///                       Constants                        ///
    //////////////////////////////////////////////////////////////

    /// @notice Current message version identifier.
    uint16 public constant MESSAGE_VERSION = 1;

    /// @notice Special address to be used as the tx origin for gas estimation calls in the
    ///         OptimismPortal and CrossDomainMessenger calls. You only need to use this address if
    ///         the minimum gas limit specified by the user is not actually enough to execute the
    ///         given message and you're attempting to estimate the actual necessary gas limit. We
    ///         use address(1) because it's the ecrecover precompile and therefore guaranteed to
    ///         never have any code on any EVM chain.
    address internal constant ESTIMATION_ADDRESS = address(1);

    /// @notice Gas reserved for finalizing the execution of `relayMessage` after the safe call.
    uint64 public constant RELAY_RESERVED_GAS = 40_000;

    /// @notice Gas reserved for the execution between the `hasMinGas` check and the external call in `relayMessage`.
    uint64 public constant RELAY_GAS_CHECK_BUFFER = 5_000;

    /// @notice This value is non-zero to reduce the gas cost of message passing transactions.
    bytes32 internal constant DEFAULT_L2_SENDER =
        bytes32(0x000000000000000000000000000000000000000000000000000000000000dEaD);

    ISolanaMessagePasser public immutable SOLANA_MESSAGE_PASSER;

    //////////////////////////////////////////////////////////////
    ///                       Events                           ///
    //////////////////////////////////////////////////////////////

    /// @notice Emitted whenever a message is sent to the other chain.
    /// @param sender       Address of the sender of the message.
    /// @param ixs          Message to trigger the recipient address with.
    /// @param messageNonce Unique nonce attached to the message.
    event SentMessage(address indexed sender, ISolanaMessagePasser.Instruction[] ixs, uint256 messageNonce);

    /// @notice Emitted whenever a message is successfully relayed on this chain.
    /// @param messageHash Hash of the message that was relayed.
    event RelayedMessage(bytes32 indexed messageHash);

    /// @notice Emitted whenever a message fails to be relayed on this chain.
    /// @param messageHash Hash of the message that failed to be relayed.
    event FailedRelayedMessage(bytes32 indexed messageHash);

    //////////////////////////////////////////////////////////////
    ///                       Storage                          ///
    //////////////////////////////////////////////////////////////

    /// @notice Messenger on the remote chain.
    ///
    /// @dev Stored as a bytes32 to handle non EVM addresses which may not fit into 20 bytes.
    bytes32 public remoteMessenger;

    /// @notice Mapping of message hashes to boolean receipt values. Note that a message will only be present in this
    ///         mapping if it has successfully been relayed on this chain, and can therefore not be relayed again.
    mapping(bytes32 messageHash => bool succeeded) public successfulMessages;

    /// @notice Mapping of message hashes to a boolean if and only if the message has failed to be executed at least
    ///         once. A message will not be present in this mapping if it successfully executed on the first attempt.
    mapping(bytes32 messageHash => bool failed) public failedMessages;

    /// @notice Nonce for the next message to be sent, without the message version applied. Use the
    ///         messageNonce getter which will insert the message version into the nonce to give you
    ///         the actual nonce to be used for the message.
    uint240 internal msgNonce;

    /// @notice Address of the message sender that interacted with the messenger on the remote chain.
    ///
    /// @dev If the value of this variable is DEFAULT_L2_SENDER then no message is currently being executed. Use the
    ///      xChainMsgSender() getter which will throw an error if this is the case.
    ///
    /// @dev Stored as a bytes32 to handle non EVM addresses which may not fit into 20 bytes.
    bytes32 private _xChainMsgSender;

    //////////////////////////////////////////////////////////////
    ///                       Public Functions                 ///
    //////////////////////////////////////////////////////////////

    /// @notice Constructs the CrossChainMessenger contract.
    constructor(ISolanaMessagePasser solanaMessagePasser) {
        SOLANA_MESSAGE_PASSER = solanaMessagePasser;
        _disableInitializers();
    }

    /// @notice Retrieves the address of the sender that interacted with the messenger on the remote chain.
    ///
    /// @dev Will throw an error if there is no message currently being executed.
    ///
    /// @return Address of the message sender that interacted with the messenger on the remote chain.
    function xChainMsgSender() external view returns (bytes32) {
        require(_xChainMsgSender != DEFAULT_L2_SENDER, "CrossChainMessenger: xChainMsgSender is not set");
        return _xChainMsgSender;
    }

    /// @notice Initializer.
    ///
    /// @param remoteMessenger_ Address of the messenger on the remote chain.
    function initialize(bytes32 remoteMessenger_) external initializer {
        // We only want to set the xChainMsgSender to the default value if it hasn't been initialized yet, meaning that
        // this is a fresh contract deployment. This prevents resetting the xChainMsgSender to the default value during
        // an upgrade, which would enable a reentrant withdrawal to sandwhich the upgrade replay a withdrawal twice.
        if (_xChainMsgSender == 0) {
            _xChainMsgSender = DEFAULT_L2_SENDER;
        }

        remoteMessenger = remoteMessenger_;
    }

    /// @notice Sends a message to some target address on the other chain. Note that if the call
    ///         always reverts, then the message will be unrelayable, and any ETH sent will be
    ///         permanently locked. The same will occur if the target on the other chain is
    ///         considered unsafe (see the _isUnsafeTarget() function).
    /// @param messageIxs Solana instructions to execute.
    function sendMessage(ISolanaMessagePasser.Instruction[] calldata messageIxs) external {
        ISolanaMessagePasser.Instruction[] memory ixs = new ISolanaMessagePasser.Instruction[](1);
        ixs[0] = ISolanaMessagePasser.Instruction({
            programId: remoteMessenger,
            accounts: new ISolanaMessagePasser.AccountMeta[](0),
            data: abi.encode(MessengerPayload({nonce: messageNonce(), sender: msg.sender, message: abi.encode(messageIxs)}))
        });

        // Triggers a message to the other messenger. Note that the amount of gas provided to the
        // message is the amount of gas requested by the user PLUS the base gas value. We want to
        // guarantee the property that the call to the target contract will always have at least
        // the minimum gas limit specified by the user.
        _sendMessage(ixs);

        emit SentMessage(msg.sender, messageIxs, messageNonce());

        unchecked {
            ++msgNonce;
        }
    }

    /// @notice Relays a message that was sent by the remote CrossChainMessenger contract. Can only be executed via
    ///         cross-chain call from the remote messenger OR if the message was already received once and is currently
    ///         being replayed.
    ///
    /// @param nonce Nonce of the message being relayed.
    /// @param sender Address of the user who sent the message.
    /// @param target Address that the message is targeted at.
    /// @param value ETH value to send with the message.
    /// @param minGasLimit Minimum amount of gas that the message can be executed with.
    /// @param message Message to send to the target.
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
        // If `_xChainMsgSender` is not the default L2 sender, this functionis being re-entered. This marks the message
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

    /// @notice Retrieves the next message nonce. Message version will be added to the upper two
    ///         bytes of the message nonce. Message version allows us to treat messages as having
    ///         different structures.
    /// @return Nonce of the next message to be sent, with added message version.
    function messageNonce() public view returns (uint256) {
        return Encoding.encodeVersionedNonce(msgNonce, MESSAGE_VERSION);
    }

    //////////////////////////////////////////////////////////////
    ///                       Internal Functions               ///
    //////////////////////////////////////////////////////////////

    /// @notice Checks whether the message is coming from the remote messenger.
    ///
    /// @return Whether the message is coming from the remote messenger.
    function _isRemoteMessenger() internal view virtual returns (bool) {
        return _bytes32ToAddress(remoteMessenger) == msg.sender;
    }

    /// @notice Checks whether a given call target is a system address that could cause the messenger to peform an
    ///         unsafe action. This is NOT a mechanism for blocking user addresses. This is ONLY used to prevent the
    ///         execution of messages to specific system addresses that could cause security issues, e.g., having the
    ///         CrossChainMessenger send messages to itself.
    ///
    /// @param target Address of the contract to check.
    ///
    /// @return Whether or not the address is an unsafe system address.
    function _isUnsafeTarget(address target) internal view virtual returns (bool) {
        return target == address(this);
    }

    /// @notice Sends a low-level message to the other messenger. Needs to be implemented by child
    ///         contracts because the logic for this depends on the network where the messenger is
    ///         being deployed.
    /// @param ixs The instructions array to be executed from the Solana MessagePasser program
    function _sendMessage(ISolanaMessagePasser.Instruction[] memory ixs) internal {
        SOLANA_MESSAGE_PASSER.initiateWithdrawal(ixs);
    }

    //////////////////////////////////////////////////////////////
    ///                       Private Functions                ///
    //////////////////////////////////////////////////////////////

    /// @notice Returns the maximum of two uint256 values.
    ///
    /// @param a First value.
    /// @param b Second value.
    ///
    /// @return Maximum of the two values.
    function _max(uint256 a, uint256 b) private pure returns (uint256) {
        return a > b ? a : b;
    }

    /// @notice Converts a bytes32 value to an address.
    ///
    /// @dev Truncates the bytes32 value to an address by taking the last 20 bytes.
    ///
    /// @param value Bytes32 value to convert.
    ///
    /// @return Converted address.
    function _bytes32ToAddress(bytes32 value) private pure returns (address) {
        return address(uint160(uint256(value)));
    }
}
