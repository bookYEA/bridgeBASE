import * as anchor from "@coral-xyz/anchor";
import {
  getOrCreateAssociatedTokenAccount,
  TOKEN_2022_PROGRAM_ID,
} from "@solana/spl-token";
import { PublicKey } from "@solana/web3.js";

import { CONSTANTS } from "../constants";

const mint = new PublicKey(CONSTANTS.solanaSpl);

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const payer = provider.wallet as anchor.Wallet;

  const userATA = await getOrCreateAssociatedTokenAccount(
    provider.connection,
    payer.payer,
    mint,
    payer.publicKey,
    false,
    undefined,
    undefined,
    TOKEN_2022_PROGRAM_ID
  );

  console.log(`User ATA: ${userATA.address.toBuffer().toString("hex")}`);
  console.log("Done!");
}

main().catch((e) => {
  console.error(e);
  console.log(e.getLogs());
});
