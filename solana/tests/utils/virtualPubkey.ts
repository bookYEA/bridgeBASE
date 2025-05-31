import { keccak256 } from "js-sha3";
import { PublicKey } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";

import type { Bridge } from "../../target/types/bridge";

import { programConstant } from "./constants";

export function virtualPubkey(name: "bridge" | "messenger"): PublicKey {
  const program = anchor.workspace.Bridge as Program<Bridge>;
  const programId = program.programId;

  const fullName = `${name}Seed` as const;
  const seed = programConstant(fullName);

  return new PublicKey(
    Buffer.from(
      keccak256(Buffer.concat([programId.toBuffer(), Buffer.from(seed)])),
      "hex"
    )
  ) as PublicKey;
}
