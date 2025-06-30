import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { toBytes } from "viem";

import type { Bridge } from "../../../target/types/bridge";
import { getConstantValue } from "../../utils/anchor-consants";

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;

  // Call parameters
  const to = toBytes("0x0000000000000000000000000000000000000000");
  const value = new anchor.BN(0);
  const data = Buffer.from("");
  const call = {
    ty: { call: {} },
    to,
    value,
    data,
  };

  const gasLimit = new anchor.BN(1_000_000); // 1M gas limit

  // Derive bridge PDA
  const bridgePda = PublicKey.findProgramAddressSync(
    [Buffer.from(getConstantValue("bridgeSeed"))],
    program.programId
  )[0];

  // Fetch bridge state to get current nonce
  const bridge = await program.account.bridge.fetch(bridgePda);
  const nonce = bridge.nonce;

  // Derive outgoing message PDA
  const outgoingMessagePda = PublicKey.findProgramAddressSync(
    [
      Buffer.from(getConstantValue("outgoingMessageSeed")),
      nonce.toBuffer("le", 8),
    ],
    program.programId
  )[0];

  console.log(`Bridge PDA: ${bridgePda.toBase58()}`);
  console.log(`Outgoing message PDA: ${outgoingMessagePda.toBase58()}`);
  console.log(`Current nonce: ${nonce.toString()}`);

  const tx = await program.methods
    .bridgeCall(gasLimit, call)
    .accountsStrict({
      payer: provider.wallet.publicKey,
      from: provider.wallet.publicKey, // Using same key as from
      gasFeeReceiver: getConstantValue("gasFeeReceiver"),
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
