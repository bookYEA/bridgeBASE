import * as anchor from "@coral-xyz/anchor";
import {
  getOrCreateAssociatedTokenAccount,
  mintTo,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { PublicKey } from "@solana/web3.js";

const mint = new PublicKey("7gpNAiU3abTrrqD4EkW5zC6Zbo3uGSzADSgsfAdQzDZ8");

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const payer = provider.wallet as anchor.Wallet;
  const amount = 100 * anchor.web3.LAMPORTS_PER_SOL;
  const receiver = new PublicKey(
    "6H3g78CLv8pzi8KTHiCbtXJyp4ggncSXdntAeSHTgBfc"
  );

  const userATA = await getOrCreateAssociatedTokenAccount(
    provider.connection,
    payer.payer,
    mint,
    receiver
  );

  console.log(`User ATA: ${userATA.address.toBuffer().toString("hex")}`);
  console.log(`Minting ${amount} tokens to ${payer.publicKey.toBase58()}`);
  await mintTo(
    provider.connection,
    payer.payer,
    mint,
    userATA.address,
    payer.publicKey,
    amount,
    [],
    undefined,
    TOKEN_PROGRAM_ID
  );
  console.log("Done!");
}

main().catch((e) => {
  console.error(e);
  console.log(e.getLogs());
});
