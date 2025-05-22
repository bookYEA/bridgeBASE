import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import type { Bridge } from "./target/types/bridge";

const REMOTE_TOKEN_ADDRESS = toArray(
  "08AF32D8482533F5C21DA4Eb99CD287dD52339F1"
); // wrapped SOL on Base Sepolia

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;

  const to = toArray("8C1a617BdB47342F9C17Ac8750E0b070c372C721");
  const value = new anchor.BN(0.001 * anchor.web3.LAMPORTS_PER_SOL);
  const minGasLimit = 100000;
  const extraData = Buffer.from("sample data payload", "utf-8");

  const tx = await program.methods
    .bridgeSolTo(REMOTE_TOKEN_ADDRESS, to, value, minGasLimit, extraData)
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

function toArray(a: string): number[] {
  return Uint8Array.from(Buffer.from(a, "hex")) as unknown as number[];
}
