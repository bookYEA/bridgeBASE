import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";

export function encodeMessengerData(
  nonce: number,
  message: Buffer,
  sender: PublicKey,
  toAddress: number[],
  minGasLimit: number
): Buffer {
  const paddingBytes = 32 - (message.length % 32);
  return Buffer.concat([
    Buffer.from([84, 170, 67, 163]), // function selector
    Buffer.from(
      Array.from({ length: 32 }, (_, i) => {
        if (i === 1) {
          return 1;
        } else if (i === 31) {
          return nonce - 1;
        }
        return 0;
      })
    ), // nonce, // nonce
    sender.toBuffer(), // sender
    Buffer.from([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, ...toAddress]), // target
    Buffer.from(new anchor.BN(0).toArray("be", 32)), // value
    Buffer.from(new anchor.BN(minGasLimit).toArray("be", 32)), // minGasLimit
    Buffer.from(Array.from({ length: 32 }, (_, i) => (i == 31 ? 192 : 0))), // message offset
    Buffer.from(new anchor.BN(Buffer.from(message).length).toArray("be", 32)), // message length
    message, // message
    Buffer.from(Array.from({ length: paddingBytes }, () => 0)), // padding to ensure message length is multiple of 32 bytes
  ]);
}
