import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import type { Bridge } from "./target/types/bridge";
import { PublicKey } from "@solana/web3.js";

const LOCAL_TOKEN_ADDRESS = new PublicKey(
  Buffer.from(
    "0501550155015501550155015501550155015501550155015501550155015501",
    "hex"
  )
);
const REMOTE_TOKEN_ADDRESS = Uint8Array.from(
  Buffer.from("2DBE6a59cA75EAaeC4FE78D0Cc8AAdAa6519Ce20", "hex")
) as unknown as number[]; // wrapped SOL on Base Sepolia

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;

  const to = Uint8Array.from(
    Buffer.from("9986ccaf9e3de0ffef82a0f7fa3a06d5afe07252", "hex")
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
