// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

/// @notice Storage layout used by this library.
///
/// @custom:storage-location erc7201:coinbase.storage.MessageStorageLib
///
/// @custom:field messages Mapping of registered message hashes.
/// @custom:field nonce Incremental nonce used per message.
struct MessageStorageLibStorage {
    mapping(bytes32 messageHash => bool registered) messages;
    uint64 nonce;
}

library MessageStorageLib {
    //////////////////////////////////////////////////////////////
    ///                       Structs                          ///
    //////////////////////////////////////////////////////////////

    /// @notice Struct representing a message to the Solana bridge.
    ///
    /// @custom:field nonce Unique nonce for the message.
    /// @custom:field sender Sender address.
    /// @custom:field data Message data to be passed to the Solana bridge.
    struct Message {
        uint64 nonce;
        address sender;
        bytes data;
    }

    //////////////////////////////////////////////////////////////
    ///                       Events                           ///
    //////////////////////////////////////////////////////////////

    /// @notice Emitted when a message is registered.
    ///
    /// @param messageHash The message's hash.
    /// @param message The message.
    event MessageRegistered(bytes32 indexed messageHash, Message message);

    //////////////////////////////////////////////////////////////
    ///                       Constants                        ///
    //////////////////////////////////////////////////////////////

    /// @dev Slot for the `MessageStorageLibStorage` struct in storage.
    ///      Computed from:
    ///         keccak256(abi.encode(uint256(keccak256("coinbase.storage.MessageStorageLib")) - 1)) &
    ///         ~bytes32(uint256(0xff))
    ///
    ///      Follows ERC-7201 (see https://eips.ethereum.org/EIPS/eip-7201).
    bytes32 private constant _MESSAGE_STORAGE_LIB_STORAGE_LOCATION =
        0x4f00c1a67879b7469d7dd58849b9cbcdedefec3f3b862c2933a36197db136100;

    //////////////////////////////////////////////////////////////
    ///                       Internal Functions               ///
    //////////////////////////////////////////////////////////////

    /// @notice Helper function to get a storage reference to the `MessageStorageLibStorage` struct.
    ///
    /// @return $ A storage reference to the `MessageStorageLibStorage` struct.
    function getMessageStorageLibStorage() internal pure returns (MessageStorageLibStorage storage $) {
        assembly ("memory-safe") {
            $.slot := _MESSAGE_STORAGE_LIB_STORAGE_LOCATION
        }
    }

    /// @notice Sends a message to the Solana bridge.
    ///
    /// @param sender The message's sender address.
    /// @param data Message data to be passed to the Solana bridge.
    function sendMessage(address sender, bytes memory data) internal {
        MessageStorageLibStorage storage $ = getMessageStorageLibStorage();

        Message memory message = Message({nonce: $.nonce, sender: sender, data: data});
        bytes32 messageHash = _hashMessage(message);
        $.messages[messageHash] = true;

        unchecked {
            ++$.nonce;
        }

        emit MessageRegistered({messageHash: messageHash, message: message});
    }

    //////////////////////////////////////////////////////////////
    ///                       Private Functions                ///
    //////////////////////////////////////////////////////////////

    /// @notice Computes the hash of a message.
    ///
    /// @param message The message to hash.
    ///
    /// @return The keccak256 hash of the encoded message.
    function _hashMessage(Message memory message) internal pure returns (bytes32) {
        return keccak256(abi.encodePacked(message.nonce, message.sender, message.data));
    }
}
