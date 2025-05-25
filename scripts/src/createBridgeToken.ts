import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import type { Bridge } from "../target/types/bridge";
import baseSepoliaAddrs from "../deployments/base_sepolia.json";
import { toArray } from "./utils/toArray";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { PublicKey } from "@solana/web3.js";

const REMOTE_TOKEN_ADDRESS = toArray(baseSepoliaAddrs.ERC20);
const DECIMALS = 9;

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;
  const [mintPda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("mint"),
      Buffer.from(REMOTE_TOKEN_ADDRESS),
      new anchor.BN(DECIMALS).toBuffer("le", 1),
    ],
    program.programId
  );
  console.log({ mintPda: mintPda.toBuffer().toString("hex") });

  const tx = await program.methods
    .createMint(REMOTE_TOKEN_ADDRESS, DECIMALS)
    .accounts({ tokenProgram: TOKEN_PROGRAM_ID })
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
