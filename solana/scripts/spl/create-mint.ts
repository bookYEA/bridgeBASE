import * as anchor from "@coral-xyz/anchor";
import { createMint, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { Keypair } from "@solana/web3.js";

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const payer = provider.wallet as anchor.Wallet;

  const mint = Keypair.generate();
  console.log(`mint: ${mint.publicKey.toBase58()}`);

  await createMint(
    provider.connection,
    payer.payer,
    payer.publicKey,
    payer.publicKey,
    10,
    mint,
    undefined,
    TOKEN_PROGRAM_ID
  );
  console.log("Done!");
}

main().catch((e) => {
  console.error(e);
  console.log(e.getLogs());
});
