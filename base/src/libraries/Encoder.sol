// SPDX-License-Identifier: MIT
pragma solidity ^0.8.15;

import {LibBit} from "solady/utils/LibBit.sol";

import {Bridge} from "../Bridge.sol";
import {CrossChainMessenger} from "../CrossChainMessenger.sol";
import {MessagePasser} from "../MessagePasser.sol";

library Encoder {
    using LibBit for uint256;

    //////////////////////////////////////////////////////////////
    ///                       Internal Functions               ///
    //////////////////////////////////////////////////////////////

    function encodeBridgePayload(Bridge.BridgePayload memory payload) internal pure returns (bytes memory) {
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

    function _serializeIxPacked(MessagePasser.Instruction memory ix) private pure returns (bytes memory) {
        bytes memory data = abi.encodePacked(ix.programId);

        for (uint256 i; i < ix.accounts.length; i++) {
            MessagePasser.AccountMeta memory account = ix.accounts[i];
            data = abi.encodePacked(data, account.pubKey, account.isWritable, account.isSigner);
        }

        return abi.encodePacked(data, ix.data);
    }

    function _getLeLength(uint256 inp) private pure returns (uint32) {
        return uint32(inp.reverseBytes() >> 224);
    }

    function _getLeAmount(uint64 amt) private pure returns (uint64) {
        return uint64(uint256(amt).reverseBytes() >> 192);
    }
}
