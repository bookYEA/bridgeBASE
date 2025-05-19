// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

// Libraries
import {Encoding} from "optimism/packages/contracts-bedrock/src/libraries/Encoding.sol";

// Interfaces
import {ISemver} from "optimism/packages/contracts-bedrock/interfaces/universal/ISemver.sol";

/// @custom:proxied true
/// @title L2ToL1MessagePasser
/// @notice The L2ToL1MessagePasser is a dedicated contract where messages that are being sent from
///         L2 to L1 can be stored. The storage root of this contract is pulled up to the top level
///         of the L2 output to reduce the cost of proving the existence of sent messages.
contract MessagePasser is ISemver {
    /// @notice Struct representing a withdrawal transaction.
    /// @custom:field nonce    Nonce of the withdrawal transaction
    /// @custom:field sender   Address of the sender of the transaction.
    struct WithdrawalTransaction {
        uint256 nonce;
        address sender;
        Instruction[] ixs;
    }

    struct AccountMeta {
        bytes32 pubKey;
        bool isSigner;
        bool isWritable;
    }

    struct Instruction {
        bytes32 programId;
        AccountMeta[] accounts;
        bytes data;
    }

    /// @notice The current message version identifier.
    uint16 public constant MESSAGE_VERSION = 1;

    /// @notice Includes the message hashes for all withdrawals
    mapping(bytes32 => bool) public sentMessages;

    /// @notice A unique value hashed with each withdrawal.
    uint240 internal msgNonce;

    /// @notice Emitted any time a withdrawal is initiated.
    /// @param nonce          Unique value corresponding to each withdrawal.
    /// @param sender         The L2 account address which initiated the withdrawal.
    /// @param withdrawalHash The hash of the withdrawal.
    event MessagePassed(uint256 indexed nonce, address indexed sender, Instruction[] ixs, bytes32 withdrawalHash);

    /// @custom:semver 1.1.2
    string public constant version = "1.1.2";

    /// @notice Sends a message from L2 to L1.
    function initiateWithdrawal(Instruction[] calldata ixs) public payable {
        bytes32 withdrawalHash =
            _hashWithdrawal(WithdrawalTransaction({nonce: messageNonce(), sender: msg.sender, ixs: ixs}));

        sentMessages[withdrawalHash] = true;

        emit MessagePassed(messageNonce(), msg.sender, ixs, withdrawalHash);

        unchecked {
            ++msgNonce;
        }
    }

    /// @notice Retrieves the next message nonce. Message version will be added to the upper two
    ///         bytes of the message nonce. Message version allows us to treat messages as having
    ///         different structures.
    /// @return Nonce of the next message to be sent, with added message version.
    function messageNonce() public view returns (uint256) {
        return Encoding.encodeVersionedNonce(msgNonce, MESSAGE_VERSION);
    }

    /// @notice Derives the withdrawal hash according to the encoding in the L2 Withdrawer contract
    /// @param _tx Withdrawal transaction to hash.
    /// @return Hashed withdrawal transaction.
    function _hashWithdrawal(WithdrawalTransaction memory _tx) internal pure returns (bytes32) {
        return keccak256(abi.encode(_tx));
    }
}
