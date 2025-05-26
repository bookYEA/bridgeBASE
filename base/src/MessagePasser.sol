// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

// Libraries
import {Encoder} from "./libraries/Encoder.sol";
import {Encoding} from "optimism/packages/contracts-bedrock/src/libraries/Encoding.sol";

/// @custom:proxied true
/// @title L2ToL1MessagePasser
/// @notice The L2ToL1MessagePasser is a dedicated contract where messages that are being sent from
///         L2 to L1 can be stored. The storage root of this contract is pulled up to the top level
///         of the L2 output to reduce the cost of proving the existence of sent messages.
contract MessagePasser {
    //////////////////////////////////////////////////////////////
    ///                       Structs                          ///
    //////////////////////////////////////////////////////////////

    /// @notice Struct representing a Solana account meta.
    ///
    /// @custom:field pubKey Public key of the account.
    /// @custom:field isSigner Whether the account is a signer.
    /// @custom:field isWritable Whether the account is writable.
    struct AccountMeta {
        bytes32 pubKey;
        bool isSigner;
        bool isWritable;
    }

    /// @notice Struct representing a Solana instruction.
    ///
    /// @custom:field programId Program ID of the instruction.
    /// @custom:field accounts Accounts used by the instruction.
    /// @custom:field data Data of the instruction.
    struct Instruction {
        bytes32 programId;
        AccountMeta[] accounts;
        bytes data;
    }

    /// @notice Struct representing a withdrawal transaction.
    ///
    /// @custom:field nonce Nonce of the withdrawal transaction.
    /// @custom:field sender Address of the sender of the transaction.
    /// @custom:field ixs Instructions to execute.
    struct WithdrawalTransaction {
        uint256 nonce;
        address sender;
        Instruction[] ixs;
    }

    //////////////////////////////////////////////////////////////
    ///                       Events                           ///
    //////////////////////////////////////////////////////////////

    /// @notice Emitted any time a withdrawal is initiated.
    ///
    /// @param nonce Unique value corresponding to each withdrawal.
    /// @param sender The L2 account address which initiated the withdrawal.
    /// @param ixs Instructions to execute.
    /// @param withdrawalHash The hash of the withdrawal.
    event MessagePassed(uint256 indexed nonce, address indexed sender, Instruction[] ixs, bytes32 withdrawalHash);

    //////////////////////////////////////////////////////////////
    ///                       Constants                        ///
    //////////////////////////////////////////////////////////////

    /// @notice The current message version identifier.
    uint16 public constant MESSAGE_VERSION = 1;

    //////////////////////////////////////////////////////////////
    ///                       Storage                          ///
    //////////////////////////////////////////////////////////////

    /// @notice Includes the message hashes for all withdrawals
    mapping(bytes32 withdrawalHash => bool sent) public sentMessages;

    /// @notice A unique value hashed with each withdrawal.
    uint240 internal _msgNonce;

    //////////////////////////////////////////////////////////////
    ///                       Public Functions                 ///
    //////////////////////////////////////////////////////////////

    /// @notice Semantic version.
    ///
    /// @custom:semver 1.1.2
    function version() external pure returns (string memory) {
        return "1.1.2";
    }

    /// @notice Sends a message from L2 to L1.
    ///
    /// @param ixs Instructions to execute.
    function initiateWithdrawal(Instruction[] calldata ixs) public payable {
        uint256 nonce = messageNonce();

        bytes32 withdrawalHash = _hashWithdrawal(WithdrawalTransaction({nonce: nonce, sender: msg.sender, ixs: ixs}));
        sentMessages[withdrawalHash] = true;

        emit MessagePassed(nonce, msg.sender, ixs, withdrawalHash);

        unchecked {
            ++_msgNonce;
        }
    }

    /// @notice Retrieves the next message nonce. Message version will be added to the upper two
    ///         bytes of the message nonce. Message version allows us to treat messages as having
    ///         different structures.
    ///
    /// @return Nonce of the next message to be sent, with added message version.
    function messageNonce() public view returns (uint256) {
        return Encoding.encodeVersionedNonce(_msgNonce, MESSAGE_VERSION);
    }

    //////////////////////////////////////////////////////////////
    ///                       Internal Functions               ///
    //////////////////////////////////////////////////////////////

    /// @notice Derives the withdrawal hash according to the encoding in the L2 Withdrawer contract
    ///
    /// @param _tx Withdrawal transaction to hash.
    ///
    /// @return Hashed withdrawal transaction.
    function _hashWithdrawal(WithdrawalTransaction memory _tx) internal pure returns (bytes32) {
        return keccak256(Encoder.encodeMessage(_tx.nonce, _tx.sender, _tx.ixs));
    }
}
