import { PublicKey } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";

import { programConstant } from "./constants";

export function encodeBridgeData(p: {
  extraData: Buffer;
  remoteToken: number[];
  localToken: PublicKey;
  from: PublicKey;
  target: number[];
  value: anchor.BN;
}): Buffer {
  const { extraData, remoteToken, localToken, from, target, value } = p;
  const extraDataPaddingBytes = 32 - (extraData.length % 32);

  const FINALIZE_BRIDGE_TOKEN_SELECTOR = programConstant(
    "finalizeBridgeTokenSelector"
  );

  return Buffer.concat([
    Buffer.from(FINALIZE_BRIDGE_TOKEN_SELECTOR), // function selector
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
