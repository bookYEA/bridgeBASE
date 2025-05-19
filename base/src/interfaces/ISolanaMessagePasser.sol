// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface ISolanaMessagePasser {
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

    event MessagePassed(
        uint256 indexed nonce,
        address indexed sender,
        address indexed target,
        uint256 value,
        uint256 gasLimit,
        bytes data,
        bytes32 withdrawalHash
    );

    function MESSAGE_VERSION() external view returns (uint16);
    function initiateWithdrawal(Instruction[] calldata) external payable;
    function messageNonce() external view returns (uint256);
    function sentMessages(bytes32) external view returns (bool);
    function version() external view returns (string memory);
}
