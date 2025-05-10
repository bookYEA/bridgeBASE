import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";

export function encodeBridgeData(
  extraData: Buffer,
  remoteToken: number[],
  localToken: PublicKey,
  from: PublicKey,
  target: number[],
  value: anchor.BN
): Buffer {
  const extraDataPaddingBytes = 32 - (extraData.length % 32);
  return Buffer.concat([
    Buffer.from("2d916920", "hex"), // function selector
    Buffer.from([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, ...remoteToken]), // remote token
    localToken.toBuffer(), // local token
    from.toBuffer(), // from
    Buffer.from([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, ...target]), // target
    Buffer.from(value.toArray("be", 32)), // value
    Buffer.from(Array.from({ length: 32 }, (_, i) => (i == 31 ? 192 : 0))), // extra_data offset
    Buffer.from(new anchor.BN(Buffer.from(extraData).length).toArray("be", 32)),
    extraData,
    Buffer.from(Array.from({ length: extraDataPaddingBytes }, () => 0)),
  ]);
}
