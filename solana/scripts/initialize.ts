import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, SystemProgram } from "@solana/web3.js";

import type { Bridge } from "../target/types/bridge";
import { confirmTransaction } from "./utils/confirm-tx";
import { getConstantValue } from "./utils/constants";

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;

  const [bridgePda] = PublicKey.findProgramAddressSync(
    [Buffer.from(getConstantValue("bridgeSeed"))],
    program.programId
  );

  console.log(`Bridge PDA: ${bridgePda.toBase58()}`);

  const tx = await program.methods
    .initialize()
    .accountsStrict({
      payer: provider.wallet.publicKey,
      bridge: bridgePda,
      systemProgram: SystemProgram.programId,
    })
    .rpc();

  console.log("Submitted transaction:", tx);

  await confirmTransaction(provider.connection, tx);
}

main().catch((e) => {
  console.error(e);
  console.log(e.getLogs());
});
