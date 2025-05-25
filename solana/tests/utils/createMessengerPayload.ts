import { expectedMessengerPubkey } from "./constants";
import { IxParam } from "./hashIxs";
import { serializeIxParam } from "./serializeIxParam";

/**
 * Helper function to create a messenger payload
 */
export function createMessengerPayload(
  nonce: number[],
  sender: number[],
  ixParam: IxParam
): IxParam {
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

  const messengerIxParam = {
    programId: expectedMessengerPubkey,
    accounts: [],
    data: serializedMessengerPayload,
  };

  return messengerIxParam;
}
