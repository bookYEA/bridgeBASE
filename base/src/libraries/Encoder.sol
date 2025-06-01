// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

import {LibBit} from "solady/utils/LibBit.sol";

import {CrossChainMessenger} from "../CrossChainMessenger.sol";
import {MessagePasser} from "../MessagePasser.sol";
import {TokenBridge} from "../TokenBridge.sol";

/// @title Encoder
///
/// @notice Library for encoding cross-chain messages and bridge payloads into binary format
///
/// @dev This library handles the serialization of various payload types for cross-chain communication.
///      It uses little-endian encoding for numeric values to ensure compatibility with target chains.
library Encoder {
    using LibBit for uint256;

    //////////////////////////////////////////////////////////////
    ///                       Internal Functions               ///
    //////////////////////////////////////////////////////////////

    /// @notice Encodes a bridge payload into a packed binary format
    ///
    /// @dev Serializes bridge transaction data in a specific order: localToken, remoteToken, from, to, amount,
    /// extraData length, extraData
    ///      Uses little-endian encoding for amount and extraData length for cross-chain compatibility
    ///
    /// @param payload The bridge payload containing token transfer information
    ///
    /// @return Encoded bytes suitable for cross-chain transmission
    function encodeBridgePayload(TokenBridge.BridgePayload memory payload) internal pure returns (bytes memory) {
        return abi.encodePacked(
            payload.localToken,
            payload.remoteToken,
            payload.from,
            payload.to,
            _getLeAmount(payload.amount),
            _getLeLength(payload.extraData.length),
            payload.extraData
        );
    }

    /// @notice Encodes a cross-chain messenger payload into binary format
    ///
    /// @dev Serializes messenger data including nonce, sender, and a variable number of instructions
    ///      Each instruction is serialized with its full metadata including account information
    ///
    /// @param payload The messenger payload containing nonce, sender, and instructions
    ///
    /// @return Encoded bytes representing the complete messenger payload
    function encodeMessengerPayload(CrossChainMessenger.MessengerPayload memory payload)
        internal
        pure
        returns (bytes memory)
    {
        bytes memory serializedIxs = abi.encodePacked(_getLeLength(payload.ixs.length));

        for (uint256 i; i < payload.ixs.length; i++) {
            serializedIxs = abi.encodePacked(serializedIxs, _serializeIx(payload.ixs[i]));
        }

        return abi.encodePacked(payload.nonce, payload.sender, _getLeLength(serializedIxs.length), serializedIxs);
    }

    /// @notice Encodes a message with nonce, sender, and instructions in packed format
    ///
    /// @dev Creates a compact encoding without length prefixes for instructions, suitable for efficient transmission
    ///      Uses packed serialization which omits metadata lengths for reduced payload size
    ///
    /// @param nonce  The message nonce for ordering and replay protection
    /// @param sender The address of the message sender
    /// @param ixs    Array of instructions to be executed on the target chain
    ///
    /// @return Encoded bytes representing the complete message
    function encodeMessage(uint256 nonce, address sender, MessagePasser.Instruction[] memory ixs)
        internal
        pure
        returns (bytes memory)
    {
        bytes memory serializedIxs = abi.encodePacked(nonce, sender);

        for (uint256 i; i < ixs.length; i++) {
            serializedIxs = abi.encodePacked(serializedIxs, _serializeIxPacked(ixs[i]));
        }

        return serializedIxs;
    }

    //////////////////////////////////////////////////////////////
    ///                       Private Functions                ///
    //////////////////////////////////////////////////////////////

    /// @notice Serializes an instruction with full metadata including length prefixes
    ///
    /// @param ix The instruction to serialize
    ///
    /// @return Serialized instruction bytes with length prefixes
    function _serializeIx(MessagePasser.Instruction memory ix) private pure returns (bytes memory) {
        bytes memory data = abi.encodePacked(ix.programId);
        data = abi.encodePacked(data, _getLeLength(ix.accounts.length));

        for (uint256 i; i < ix.accounts.length; i++) {
            MessagePasser.AccountMeta memory account = ix.accounts[i];
            data = abi.encodePacked(data, account.pubKey, account.isWritable, account.isSigner);
        }

        data = abi.encodePacked(data, _getLeLength(ix.data.length), ix.data);

        return data;
    }

    /// @notice Serializes an instruction in packed format without length prefixes
    ///
    /// @param ix The instruction to serialize
    ///
    /// @return Packed instruction bytes without length metadata
    function _serializeIxPacked(MessagePasser.Instruction memory ix) private pure returns (bytes memory) {
        bytes memory data = abi.encodePacked(ix.programId);

        for (uint256 i; i < ix.accounts.length; i++) {
            MessagePasser.AccountMeta memory account = ix.accounts[i];
            data = abi.encodePacked(data, account.pubKey, account.isWritable, account.isSigner);
        }

        return abi.encodePacked(data, ix.data);
    }

    /// @notice Converts a length value to little-endian 32-bit format
    ///
    /// @param inp The input length as uint256
    ///
    /// @return Little-endian encoded length as uint32
    function _getLeLength(uint256 inp) private pure returns (uint32) {
        return uint32(inp.reverseBytes() >> 224);
    }

    /// @notice Converts an amount value to little-endian 64-bit format
    ///
    /// @param amt The input amount as uint64
    ///
    /// @return Little-endian encoded amount as uint64
    function _getLeAmount(uint64 amt) private pure returns (uint64) {
        return uint64(uint256(amt).reverseBytes() >> 192);
    }
}
