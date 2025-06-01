// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

import {Encoding} from "optimism/packages/contracts-bedrock/src/libraries/Encoding.sol";

import {Encoder} from "./libraries/Encoder.sol";

/// @title MessagePasser
///
/// @notice The MessagePasser is a dedicated contract for initiating withdrawals to Solana. Messages sent through this
///         contract contain Solana instructions that will be executed on the Solana network.
contract MessagePasser {
    //////////////////////////////////////////////////////////////
    ///                       Structs                          ///
    //////////////////////////////////////////////////////////////

    /// @notice Struct representing a Solana account meta, which defines the permissions and role of an account in a
    ///         Solana transaction.
    ///
    /// @custom:field pubKey     Public key of the Solana account.
    /// @custom:field isSigner   Whether the account must sign the transaction.
    /// @custom:field isWritable Whether the account data can be modified during execution.
    struct AccountMeta {
        bytes32 pubKey;
        bool isSigner;
        bool isWritable;
    }

    /// @notice Struct representing a Solana instruction to be executed.
    ///
    /// @custom:field programId The Solana program ID that will process this instruction.
    /// @custom:field accounts  Array of account metas that define which accounts the instruction will use.
    /// @custom:field data      The instruction data that will be passed to the Solana program.
    struct Instruction {
        bytes32 programId;
        AccountMeta[] accounts;
        bytes data;
    }

    /// @notice Struct representing a complete withdrawal transaction containing Solana instructions.
    ///
    /// @custom:field nonce  Unique identifier for this withdrawal transaction.
    /// @custom:field sender Ethereum address that initiated the withdrawal.
    /// @custom:field ixs    Array of Solana instructions to be executed as part of this withdrawal.
    struct WithdrawalTransaction {
        uint256 nonce;
        address sender;
        Instruction[] ixs;
    }

    //////////////////////////////////////////////////////////////
    ///                       Events                           ///
    //////////////////////////////////////////////////////////////

    /// @notice Emitted when a withdrawal to Solana is initiated.
    ///
    /// @param nonce          Unique nonce for this withdrawal transaction.
    /// @param sender         The Ethereum address that initiated the withdrawal.
    /// @param ixs            Array of Solana instructions to be executed.
    /// @param withdrawalHash The hash of the complete withdrawal transaction.
    event MessagePassed(uint256 indexed nonce, address indexed sender, Instruction[] ixs, bytes32 withdrawalHash);

    //////////////////////////////////////////////////////////////
    ///                       Constants                        ///
    //////////////////////////////////////////////////////////////

    /// @notice The current message version identifier used for encoding versioned nonces.
    uint16 public constant MESSAGE_VERSION = 1;

    //////////////////////////////////////////////////////////////
    ///                       Storage                          ///
    //////////////////////////////////////////////////////////////

    /// @notice Tracks whether a withdrawal hash has been processed to prevent replay attacks.
    mapping(bytes32 withdrawalHash => bool sent) public sentMessages;

    /// @notice Internal counter for generating unique nonces for each withdrawal. Uses uint240 to leave space for
    ///         version encoding in the upper bits.
    uint240 internal _msgNonce;

    //////////////////////////////////////////////////////////////
    ///                       Public Functions                 ///
    //////////////////////////////////////////////////////////////

    /// @notice Returns the semantic version of this contract.
    function version() external pure returns (string memory) {
        return "1.1.2";
    }

    /// @notice Retrieves the next message nonce with version encoding. The message version is encoded in the upper two
    ///         bytes of the nonce, allowing for different message structures in future versions.
    ///
    /// @return The next nonce to be used, with the message version encoded in the upper bits.
    function messageNonce() public view returns (uint256) {
        return Encoding.encodeVersionedNonce(_msgNonce, MESSAGE_VERSION);
    }

    /// @notice Initiates a withdrawal to Solana by recording Solana instructions to be executed. This function creates
    ///         a withdrawal transaction, hashes it for verification, and emits an event that can be monitored by
    ///         offchain relayers.
    ///
    /// @param ixs Array of Solana instructions to be executed as part of the withdrawal.
    function initiateWithdrawal(Instruction[] calldata ixs) public payable {
        uint256 nonce = messageNonce();

        bytes32 withdrawalHash = _hashWithdrawal(WithdrawalTransaction({nonce: nonce, sender: msg.sender, ixs: ixs}));
        sentMessages[withdrawalHash] = true;

        emit MessagePassed(nonce, msg.sender, ixs, withdrawalHash);

        unchecked {
            ++_msgNonce;
        }
    }

    //////////////////////////////////////////////////////////////
    ///                       Internal Functions               ///
    //////////////////////////////////////////////////////////////

    /// @notice Computes the hash of a withdrawal transaction for verification and storage. Uses the same encoding
    ///         format as expected by relayer and verification systems.
    ///
    /// @param withdrawal The withdrawal transaction to hash.
    ///
    /// @return The keccak256 hash of the encoded withdrawal transaction.
    function _hashWithdrawal(WithdrawalTransaction memory withdrawal) internal pure returns (bytes32) {
        return keccak256(Encoder.encodeMessage(withdrawal.nonce, withdrawal.sender, withdrawal.ixs));
    }
}
