import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, SystemProgram, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { toBytes } from "viem";

import type { Bridge } from "../../../target/types/bridge";
import { getConstantValue } from "../../utils/anchor-consants";

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;

  // Bridge parameters
  const to = toBytes("0x25f7fD8f50D522b266764cD3b230EDaA8CbB9f75"); // Recipient on Base
  const remoteToken = toBytes("0xfDaB33bcbD3801BE97056c3541cEC59760E23a3B"); // Wrapped SOL address on Base
  const amount = new anchor.BN(1); // 0.000000001 SOL
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

  // Derive SOL vault PDA
  const solVaultPda = PublicKey.findProgramAddressSync(
    [Buffer.from(getConstantValue("solVaultSeed")), Buffer.from(remoteToken)],
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

  console.log(`Bridge PDA: ${bridgePda.toBase58()}`);
  console.log(`SOL Vault PDA: ${solVaultPda.toBase58()}`);
  console.log(`Outgoing message PDA: ${outgoingMessagePda.toBase58()}`);
  console.log(`Current nonce: ${nonce.toString()}`);
  console.log(`Bridging ${amount.toNumber() / LAMPORTS_PER_SOL} SOL`);

  const tx = await program.methods
    .bridgeSol(gasLimit, to, remoteToken, amount, call)
    .accountsStrict({
      payer: provider.wallet.publicKey,
      from: provider.wallet.publicKey,
      gasFeeReceiver: getConstantValue("gasFeeReceiver"),
      solVault: solVaultPda,
      bridge: bridgePda,
      outgoingMessage: outgoingMessagePda,
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
