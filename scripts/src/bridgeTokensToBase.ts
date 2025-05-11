import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import type { Bridge } from "../target/types/bridge";
import { PublicKey } from "@solana/web3.js";

const mint = new PublicKey("EpGUaQN3ndd6LvY66kh4NxiStwmZHoApZWtwRMmn5SVS");
const REMOTE_TOKEN_ADDRESS = toArray(
  "dE97F9F11FF70335528631a6783FB8fC1b7996F9"
); // wrapped SPL on Base Sepolia

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;

  const to = toArray("9986ccaf9e3de0ffef82a0f7fa3a06d5afe07252");
  const value = new anchor.BN(0.001 * anchor.web3.LAMPORTS_PER_SOL);
  const minGasLimit = 100000;
  const extraData = Buffer.from("sample data payload", "utf-8");

  const tx = await program.methods
    .bridgeTokensTo(REMOTE_TOKEN_ADDRESS, to, value, minGasLimit, extraData)
    .accounts({ mint })
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

function toArray(a: string): number[] {
  return Uint8Array.from(Buffer.from(a, "hex")) as unknown as number[];
}
