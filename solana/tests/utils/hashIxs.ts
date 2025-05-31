import * as sha3 from "js-sha3";
import { Program } from "@coral-xyz/anchor";
import { Bridge } from "../../target/types/bridge";
import { toNumberArray } from "./toNumberArray";

export type IxParam = Parameters<
  Program<Bridge>["methods"]["proveTransaction"]
>[3][number];

export function hashIxs(p: {
  nonce: number[];
  remoteSender: number[];
  ixs: IxParam[];
}) {
  const { nonce, remoteSender, ixs } = p;
  let data = Buffer.alloc(0);

  data = Buffer.concat([data, Buffer.from(nonce)]);
  data = Buffer.concat([data, Buffer.from(remoteSender)]);

  // Add each instruction
  for (const ix of ixs) {
    data = Buffer.concat([data, ix.programId.toBuffer()]);
    for (const account of ix.accounts) {
      data = Buffer.concat([data, account.pubkey.toBuffer()]);
      data = Buffer.concat([data, Buffer.from([account.isWritable ? 1 : 0])]);
      data = Buffer.concat([data, Buffer.from([account.isSigner ? 1 : 0])]);
    }
    data = Buffer.concat([data, Buffer.from(ix.data)]);
  }

  // Return the keccak256 hash
  return toNumberArray(sha3.keccak256(data));
}
