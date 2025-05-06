import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import type { Bridge } from "./target/types/bridge";
import { PublicKey } from "@solana/web3.js";

const LOCAL_TOKEN_ADDRESS = PublicKey.default;
const REMOTE_TOKEN_ADDRESS = Uint8Array.from(
  Buffer.from("E398D7afe84A6339783718935087a4AcE6F6DFE8", "hex")
) as unknown as number[]; // random address for testing

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;

  const to = Uint8Array.from(
    Buffer.from("0x9986ccaf9e3de0ffef82a0f7fa3a06d5afe07252", "hex")
  ) as unknown as number[];
  const value = new anchor.BN(0.001 * anchor.web3.LAMPORTS_PER_SOL);
  const minGasLimit = 100000;
  const extraData = Buffer.from("sample data payload", "utf-8");

  const tx = await program.methods
    .bridgeTokensTo(
      LOCAL_TOKEN_ADDRESS,
      REMOTE_TOKEN_ADDRESS,
      to,
      value,
      minGasLimit,
      extraData
    )
    .accounts({})
    .rpc();

  console.log("Deposit transaction signature", tx);
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

main().catch((e) => {
  console.error(e);
  console.log(e.getLogs());
});
