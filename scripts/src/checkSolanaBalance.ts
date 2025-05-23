import * as anchor from "@coral-xyz/anchor";
import {
  getAccount,
  getOrCreateAssociatedTokenAccount,
} from "@solana/spl-token";
import { PublicKey } from "@solana/web3.js";
import { loadFromEnv } from "./utils/loadFromEnv";

const mint = new PublicKey(loadFromEnv("MINT"));

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

  const account = await getAccount(provider.connection, userATA.address);
  console.log(
    `Account token balance: ${
      Number(account.amount) / anchor.web3.LAMPORTS_PER_SOL
    }`
  );
}

main().catch(console.error);
