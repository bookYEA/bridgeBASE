// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

import {Bridge} from "../Bridge.sol";
import {CrossChainMessenger} from "../CrossChainMessenger.sol";
import {ISolanaMessagePasser} from "../interfaces/ISolanaMessagePasser.sol";

library Encoder {
    function encodeBridgePayload(Bridge.BridgePayload memory payload) internal pure returns (bytes memory) {
        return abi.encodePacked(
            payload.localToken,
            payload.remoteToken,
            payload.from,
            payload.to,
            payload.amount,
            uint32(payload.extraData.length),
            payload.extraData
        );
    }

    function encodeMessengerPayload(CrossChainMessenger.MessengerPayload memory payload)
        internal
        pure
        returns (bytes memory)
    {
        bytes memory serializedIxs = abi.encodePacked(uint32(payload.ixs.length));

        for (uint256 i; i < payload.ixs.length; i++) {
            serializedIxs = abi.encodePacked(serializedIxs, serializeIx(payload.ixs[i]));
        }

        return abi.encodePacked(payload.nonce, payload.sender, uint32(serializedIxs.length), serializedIxs);
    }

    function encodeMessage(uint256 nonce, address sender, ISolanaMessagePasser.Instruction[] memory ixs)
        internal
        pure
        returns (bytes memory)
    {
        bytes memory serializedIxs = abi.encodePacked(nonce, sender);

        for (uint256 i; i < ixs.length; i++) {
            serializedIxs = abi.encodePacked(serializedIxs, serializeIxPacked(ixs[i]));
        }

        return serializedIxs;
    }

    function serializeIx(ISolanaMessagePasser.Instruction memory ix) internal pure returns (bytes memory) {
        bytes memory data = abi.encodePacked(ix.programId);
        data = abi.encodePacked(data, uint32(ix.accounts.length));

        for (uint256 i; i < ix.accounts.length; i++) {
            ISolanaMessagePasser.AccountMeta memory account = ix.accounts[i];
            data = abi.encodePacked(data, account.pubKey, account.isWritable, account.isSigner);
        }

        data = abi.encodePacked(data, uint32(ix.data.length), ix.data);

        return data;
    }

    function serializeIxPacked(ISolanaMessagePasser.Instruction memory ix) internal pure returns (bytes memory) {
        bytes memory data = abi.encodePacked(ix.programId);

        for (uint256 i; i < ix.accounts.length; i++) {
            ISolanaMessagePasser.AccountMeta memory account = ix.accounts[i];
            data = abi.encodePacked(data, account.pubKey, account.isWritable, account.isSigner);
        }

        return abi.encodePacked(data, ix.data);
    }
}
