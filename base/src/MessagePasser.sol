// SPDX-License-Identifier: MIT
pragma solidity 0.8.28;

/// @title MessagePasser
///
/// @notice The MessagePasser is a dedicated contract for initiating withdrawals to Solana. Messages sent through this
///         contract contain Solana instructions that will be executed on the Solana network.
contract MessagePasser {
    //////////////////////////////////////////////////////////////
    ///                       Structs                          ///
    //////////////////////////////////////////////////////////////

    /// @notice Struct representing a remote call to a Solana program.
    ///
    /// @custom:field nonce Unique identifier for this remote call.
    /// @custom:field sender Ethereum address that initiated the remote call.
    /// @custom:field data Data to be passed to the Solana program.
    struct RemoteCall {
        uint256 nonce;
        address sender;
        bytes data;
    }

    //////////////////////////////////////////////////////////////
    ///                       Events                           ///
    //////////////////////////////////////////////////////////////

    /// @notice Emitted when a remote call is initiated.
    ///
    /// @param nonce Unique nonce for this remote call.
    /// @param sender The Ethereum address that initiated the remote call.
    /// @param data Data to be passed to the Solana program.
    /// @param remoteCallHash The hash of the complete remote call.
    event RemoteCallSent(uint256 indexed nonce, address indexed sender, bytes data, bytes32 remoteCallHash);

    //////////////////////////////////////////////////////////////
    ///                       Storage                          ///
    //////////////////////////////////////////////////////////////

    /// @notice Tracks whether a remote call hash has been processed to prevent replay attacks.
    mapping(bytes32 remoteCallHash => bool sent) public sentRemoteCalls;

    /// @notice Internal counter for generating unique nonces for each remote call.
    uint256 internal _remoteCallNonce;

    //////////////////////////////////////////////////////////////
    ///                       Public Functions                 ///
    //////////////////////////////////////////////////////////////

    /// @notice Returns the semantic version of this contract.
    function version() external pure returns (string memory) {
        return "1.1.2";
    }

    /// @notice Sends a remote call to a Solana program. This function creates a remote call, hashes it for
    ///         verification, and emits an event that can be monitored by offchain relayers.
    ///
    /// @param data Data to be passed to the Solana program.
    function sendRemoteCall(bytes calldata data) public payable {
        uint256 nonce = _remoteCallNonce;

        bytes32 remoteCallHash = _hashRemoteCall(RemoteCall({nonce: nonce, sender: msg.sender, data: data}));
        sentRemoteCalls[remoteCallHash] = true;

        emit RemoteCallSent(nonce, msg.sender, data, remoteCallHash);

        unchecked {
            ++_remoteCallNonce;
        }
    }

    //////////////////////////////////////////////////////////////
    ///                       Internal Functions               ///
    //////////////////////////////////////////////////////////////

    /// @notice Computes the hash of a remote call for verification and storage. Uses the same encoding format as
    ///         expected by relayer and verification systems.
    ///
    /// @param remoteCall The remote call to hash.
    ///
    /// @return The keccak256 hash of the encoded remote call.
    function _hashRemoteCall(RemoteCall memory remoteCall) internal pure returns (bytes32) {
        return keccak256(abi.encodePacked(remoteCall.nonce, remoteCall.sender, remoteCall.data));
    }
}
