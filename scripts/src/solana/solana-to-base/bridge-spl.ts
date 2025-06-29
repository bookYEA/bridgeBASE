import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  getAssociatedTokenAddressSync,
  TOKEN_2022_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { PublicKey, SystemProgram, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { toBytes } from "viem";

import type { Bridge } from "../../../target/types/bridge";
import { getConstantValue } from "../../utils/anchor-consants";

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;

  // Bridge parameters
  const mint = new PublicKey("EpGUaQN3ndd6LvY66kh4NxiStwmZHoApZWtwRMmn5SVS");
  const to = toBytes("0x25f7fD8f50D522b266764cD3b230EDaA8CbB9f75"); // Recipient on Base
  const remoteToken = toBytes("0x8cB2961F96325c64572a67d855ebAB50A3c8332D"); // USDC on Base
  const amount = new anchor.BN(1);
  const call = null; // No call for this example

  const gasLimit = new anchor.BN(1_000_000); // 1M gas limit

  // Derive bridge PDA
  const bridgePda = PublicKey.findProgramAddressSync(
    [Buffer.from(getConstantValue("bridgeSeed"))],
    program.programId
  )[0];

  // Fetch bridge state to get current nonce
  const bridge = await program.account.bridge.fetch(bridgePda);
  const nonce = bridge.nonce;

  // Derive token vault PDA
  const tokenVaultPda = PublicKey.findProgramAddressSync(
    [
      Buffer.from(getConstantValue("tokenVaultSeed")),
      mint.toBuffer(),
      remoteToken,
    ],
    program.programId
  )[0];

  // Derive outgoing message PDA
  const outgoingMessagePda = PublicKey.findProgramAddressSync(
    [
      Buffer.from(getConstantValue("outgoingMessageSeed")),
      nonce.toBuffer("le", 8),
    ],
    program.programId
  )[0];

  // Get user's token account
  const fromTokenAccount = getAssociatedTokenAddressSync(
    mint,
    provider.wallet.publicKey,
    false,
    TOKEN_PROGRAM_ID
  );

  console.log(`Bridge PDA: ${bridgePda.toBase58()}`);
  console.log(`Token Vault PDA: ${tokenVaultPda.toBase58()}`);
  console.log(`Outgoing message PDA: ${outgoingMessagePda.toBase58()}`);
  console.log(`From token account: ${fromTokenAccount.toBase58()}`);
  console.log(`Current nonce: ${nonce.toString()}`);
  console.log(`Bridging amount: ${amount.toNumber()}`);

  const tx = await program.methods
    .bridgeSpl(gasLimit, Array.from(to), Array.from(remoteToken), amount, call)
    .accountsStrict({
      payer: provider.wallet.publicKey,
      from: provider.wallet.publicKey,
      gasFeeReceiver: getConstantValue("gasFeeReceiver"),
      mint: mint,
      fromTokenAccount: fromTokenAccount,
      tokenVault: tokenVaultPda,
      bridge: bridgePda,
      outgoingMessage: outgoingMessagePda,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    })
    .rpc();

  console.log("Submitted transaction:", tx);

  const latestBlockHash = await provider.connection.getLatestBlockhash();
  await provider.connection.confirmTransaction(
    {
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: tx,
    },
    "confirmed"
  );

  console.log("Confirmed transaction:", tx);
}

main().catch((e) => {
  console.error(e);
  console.log(e.getLogs());
});
