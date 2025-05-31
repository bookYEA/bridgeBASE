import { PublicKey } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";

import { IxParam } from "./hashIxs";
import { serializeIxParam } from "./serializeIxParam";
import { virtualPubkey } from "./virtualPubkey";

/**
 * Helper function to create a messenger ix param
 */
export function createMessengerIxParam(p: {
  nonce: number[];
  sender: number[];
  ixParam: IxParam;
}): IxParam {
  const { nonce, sender, ixParam } = p;
  const serializedIxParam = serializeIxParam(ixParam);

  // Construct the Vec<Ix> payload for MessengerPayload.message
  // If MessengerPayload.message is meant to be a Vec<Ix> containing one Ix,
  // it needs to be serialized as: [length_of_Vec<Ix> (u32le), serialized_bytes_of_Ix_0]
  const vecIxLengthBuffer = Buffer.alloc(4);
  vecIxLengthBuffer.writeUInt32LE(1, 0); // We have 1 instruction in this vector
  const message = Buffer.concat([vecIxLengthBuffer, serializedIxParam]);

  // Serialize MessengerPayload
  // Fields: nonce: [u8; 32], sender: [u8; 20], message: Vec<u8>
  const nonceBuffer = Buffer.from(nonce);
  const senderBuffer = Buffer.from(sender);

  // For MessengerPayload.message (which is itself a Vec<u8> containing the serialized Vec<Ix>)
  // Borsh expects u32 length prefix + data for this outer Vec<u8>
  const messengerPayloadMessageLenBuffer = Buffer.alloc(4);
  messengerPayloadMessageLenBuffer.writeUInt32LE(message.length, 0);

  const serializedMessengerPayload = Buffer.concat([
    nonceBuffer, // 32 bytes
    senderBuffer, // 20 bytes
    messengerPayloadMessageLenBuffer, // 4 bytes (length of message)
    message, // actual bytes of message (serialized Vec<Ix>)
  ]);

  const messengerVirtualPubkey = virtualPubkey("messenger");

  const messengerIxParam = {
    programId: messengerVirtualPubkey,
    accounts: [],
    data: serializedMessengerPayload,
  };

  return messengerIxParam;
}

export function createBridgeIxParam(p: {
  localToken: PublicKey;
  remoteToken: number[];
  from: number[];
  to: PublicKey;
  amount: anchor.BN;
  extraData: Buffer;
}): IxParam {
  const { localToken, remoteToken, from, to, amount, extraData } = p;
  // Serialize BridgePayload
  // Fields: local_token: Pubkey, remote_token: [u8; 20], from: [u8; 20], to: Pubkey, amount: u64, extra_data: Vec<u8>
  const localTokenBuffer = localToken.toBuffer();
  const remoteTokenBuffer = Buffer.from(remoteToken);
  const fromBuffer = Buffer.from(from);
  const toBuffer = to.toBuffer();
  const amountBuffer = amount.toBuffer("le", 8);

  const extraDataLenBuffer = Buffer.alloc(4);
  extraDataLenBuffer.writeUint32LE(extraData.length, 0);

  const serializedBridgePayload = Buffer.concat([
    localTokenBuffer,
    remoteTokenBuffer,
    fromBuffer,
    toBuffer,
    amountBuffer,
    extraDataLenBuffer,
    extraData,
  ]);

  const virtualBridgePubkey = virtualPubkey("bridge");

  const bridgeIxParam = {
    programId: virtualBridgePubkey,
    accounts: [],
    data: serializedBridgePayload,
  };

  return bridgeIxParam;
}
