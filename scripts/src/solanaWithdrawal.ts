import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import type { Bridge } from "../target/types/bridge";
import { PublicKey } from "@solana/web3.js";

export type IxParam = Parameters<
  Program<Bridge>["methods"]["proveTransaction"]
>[3][number];

export async function main(
  nonce: number[],
  transactionHash: number[],
  remoteSender: number[],
  ixs: IxParam[],
  leafIndex: number,
  blockNumber: number
) {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;

  const oracleUrl = `http://localhost:8080/proof/${leafIndex}`;
  const res = await fetch(oracleUrl);
  const json = await res.json();

  const [rootPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("output_root"), new anchor.BN(blockNumber).toBuffer("le", 8)],
    program.programId
  );

  const tx = await program.methods
    .proveTransaction(
      transactionHash,
      nonce,
      remoteSender,
      ixs,
      json.proof.map((element: string) =>
        Array.from(Buffer.from(element, "base64"))
      ),
      new anchor.BN(leafIndex),
      new anchor.BN(json.totalLeafCount)
    )
    .accounts({ root: rootPda })
    .rpc();

  console.log("Prove transaction signature", tx);
  const latestBlockHash = await provider.connection.getLatestBlockhash();
  await provider.connection.confirmTransaction(
    {
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: tx,
    },
    "confirmed"
  );
}
