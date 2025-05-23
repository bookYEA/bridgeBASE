import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import type { Bridge } from "../target/types/bridge";
import { PublicKey } from "@solana/web3.js";
import baseSepoliaAddrs from "../deployments/base_sepolia.json";
import { toArray } from "./utils/toArray";
import { loadFromEnv } from "./utils/loadFromEnv";

const mint = new PublicKey(loadFromEnv("MINT"));
const REMOTE_TOKEN_ADDRESS = toArray(baseSepoliaAddrs.WrappedSPL);

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;

  const to = toArray(loadFromEnv("USER"));
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
