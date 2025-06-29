import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { TOKEN_2022_PROGRAM_ID } from "@solana/spl-token";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { keccak256, toBytes } from "viem";

import type { Bridge } from "../../../target/types/bridge";
import { getConstantValue } from "../../utils/anchor-consants";

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;

  // Token metadata
  const decimals = 6;
  const metadata = {
    name: "Wrapped ETH",
    symbol: "wETH",
    uri: "https://example.com/metadata.json",
    remoteToken: toBytes("0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE"), // Native ETH address on Base
    scalerExponent: 6,
  };

  const gasLimit = new anchor.BN(1_000_000); // 1M gas limit

  const metadataHash = keccak256(
    Buffer.concat([
      Buffer.from(metadata.name),
      Buffer.from(metadata.symbol),
      Buffer.from(metadata.remoteToken),
      new anchor.BN(metadata.scalerExponent).toBuffer("le", 1),
    ])
  );

  // Derive mint PDA
  const mintPda = PublicKey.findProgramAddressSync(
    [
      Buffer.from(getConstantValue("wrappedTokenSeed")),
      Buffer.from([decimals]),
      toBytes(metadataHash),
    ],
    program.programId
  )[0];

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
  console.log(`Mint PDA: ${mintPda.toBase58()}`);
  console.log(`Outgoing message PDA: ${outgoingMessagePda.toBase58()}`);

  const tx = await program.methods
    .wrapToken(decimals, metadata, gasLimit)
    .accountsStrict({
      payer: provider.wallet.publicKey,
      gasFeeReceiver: getConstantValue("gasFeeReceiver"),
      mint: mintPda,
      bridge: bridgePda,
      outgoingMessage: outgoingMessagePda,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
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
