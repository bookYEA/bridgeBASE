// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import {LibBit} from "solady/utils/LibBit.sol";

/// @notice Represents a Solana public key (32 bytes)
type Pubkey is bytes32;

function eq(Pubkey a, Pubkey b) pure returns (bool) {
    return Pubkey.unwrap(a) == Pubkey.unwrap(b);
}

using {eq as ==} for Pubkey global;

/// @notice Program Derived Address specification.
///
/// @param seeds Array of seed bytes for PDA generation
/// @param programId The program that owns this PDA
struct Pda {
    bytes[] seeds;
    Pubkey programId;
}

/// @notice Enum for pubkey or PDA variants
enum PubkeyOrPdaVariant {
    Pubkey,
    PDA
}

/// @notice Union type for either a direct pubkey or PDA
///
/// @param variant The type of key (Pubkey or PDA)
/// @param variantData Serialized data for the variant
struct PubkeyOrPda {
    PubkeyOrPdaVariant variant;
    bytes variantData;
}

/// @notice Account metadata for Solana instruction
///
/// @param pubkey The account's public key or PDA
/// @param isWritable Whether the account is writable
/// @param isSigner Whether the account is a signer
struct IxAccount {
    PubkeyOrPda pubkey;
    bool isWritable;
    bool isSigner;
}

/// @notice Solana instruction structure
///
/// @param programId The program to execute
/// @param accounts Array of accounts required by the instruction
/// @param data Instruction data payload
struct Ix {
    Pubkey programId;
    string name;
    IxAccount[] accounts;
    bytes data;
}

/// @title SVMLib - Solana Virtual Machine library for Solidity
///
/// @notice Provides types and serialization for Solana instructions with Borsh compatibility
library SVMLib {
    using LibBit for uint256;

    //////////////////////////////////////////////////////////////
    ///                       Internal Functions               ///
    //////////////////////////////////////////////////////////////

    /// @notice Creates an account with a direct pubkey.
    ///
    /// @param pubkey The public key of the account
    /// @param isWritable Whether the account should be writable
    /// @param isSigner Whether the account should be a signer
    /// @return IxAccount with the specified parameters
    function createPubkeyAccount(Pubkey pubkey, bool isWritable, bool isSigner)
        internal
        pure
        returns (IxAccount memory)
    {
        return IxAccount({
            pubkey: PubkeyOrPda({variant: PubkeyOrPdaVariant.Pubkey, variantData: abi.encodePacked(pubkey)}),
            isWritable: isWritable,
            isSigner: isSigner
        });
    }

    /// @notice Creates an account with a Program Derived Address.
    ///
    /// @param pda The PDA specification
    /// @param isWritable Whether the account should be writable
    /// @param isSigner Whether the account should be a signer
    /// @return IxAccount with the specified PDA parameters
    function createPdaAccount(Pda memory pda, bool isWritable, bool isSigner)
        internal
        pure
        returns (IxAccount memory)
    {
        bytes memory data = abi.encodePacked(_getLeLength(pda.seeds.length));
        for (uint256 i; i < pda.seeds.length; i++) {
            data = abi.encodePacked(data, _serializeBytes(pda.seeds[i]));
        }

        data = abi.encodePacked(data, pda.programId);

        return IxAccount({
            pubkey: PubkeyOrPda({variant: PubkeyOrPdaVariant.PDA, variantData: data}),
            isWritable: isWritable,
            isSigner: isSigner
        });
    }

    /// @notice Serializes a list of Solana instructions to Borsh-compatible bytes.
    ///
    /// @param ixs The list of instructions to serialize
    ///
    /// @return Serialized instruction bytes ready for Solana deserialization
    function serializeAnchorIxs(Ix[] memory ixs) internal pure returns (bytes memory) {
        bytes memory result = abi.encodePacked(_getLeLength(ixs.length));
        for (uint256 i; i < ixs.length; i++) {
            result = abi.encodePacked(result, serializeAnchorIx(ixs[i]));
        }

        return result;
    }

    /// @notice Serializes a Solana instruction to Borsh-compatible bytes.
    ///
    /// @param ix The instruction to serialize
    ///
    /// @return Serialized instruction bytes ready for Solana deserialization
    function serializeAnchorIx(Ix memory ix) internal pure returns (bytes memory) {
        bytes memory result = abi.encodePacked(ix.programId);

        // Serialize accounts array
        result = abi.encodePacked(result, _getLeLength(ix.accounts.length));
        for (uint256 i = 0; i < ix.accounts.length; i++) {
            result = abi.encodePacked(result, uint8(ix.accounts[i].pubkey.variant));
            result = abi.encodePacked(result, ix.accounts[i].pubkey.variantData);
            // Serialize account flags
            result = abi.encodePacked(result, ix.accounts[i].isWritable ? uint8(1) : uint8(0));
            result = abi.encodePacked(result, ix.accounts[i].isSigner ? uint8(1) : uint8(0));
        }

        // Serialize instruction data
        bytes32 ixDiscriminator = sha256(abi.encodePacked("global:", ix.name));
        bytes memory ixData = abi.encodePacked(bytes8(ixDiscriminator), ix.data);
        result = abi.encodePacked(result, _serializeBytes(ixData));

        return result;
    }

    function toLittleEndian(uint256 value) internal pure returns (uint64) {
        return uint64(value.reverseBytes() >> 192);
    }

    //////////////////////////////////////////////////////////////
    ///                       Private Functions                ///
    //////////////////////////////////////////////////////////////

    /// @dev Serializes bytes with length prefix
    function _serializeBytes(bytes memory data) private pure returns (bytes memory) {
        return abi.encodePacked(_getLeLength(data.length), data);
    }

    /// @notice Converts a length value to little-endian 32-bit format
    ///
    /// @param inp The input length as uint256
    /// @return Little-endian encoded length as uint32
    function _getLeLength(uint256 inp) private pure returns (uint32) {
        return uint32(inp.reverseBytes() >> 224);
    }
}
