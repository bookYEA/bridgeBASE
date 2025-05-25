import * as anchor from "@coral-xyz/anchor";
import { getOrCreateAssociatedTokenAccount } from "@solana/spl-token";
import { PublicKey } from "@solana/web3.js";
import { loadFromEnv } from "./utils/loadFromEnv";

const mint = new PublicKey(loadFromEnv("ERC20_MINT"));

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const payer = provider.wallet as anchor.Wallet;

  const userATA = await getOrCreateAssociatedTokenAccount(
    provider.connection,
    payer.payer,
    mint,
    payer.publicKey
  );

  console.log(`User ATA: ${userATA.address.toBuffer().toString("hex")}`);
  console.log("Done!");
}

main().catch((e) => {
  console.error(e);
  console.log(e.getLogs());
});
