import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { TOKEN_2022_PROGRAM_ID } from "@solana/spl-token";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { keccak256, toBytes } from "viem";

import type { Bridge } from "../../target/types/bridge";
import { confirmTransaction } from "../utils/confirm-tx";
import { getConstantValue } from "../utils/constants";
import { CONSTANTS } from "../constants";

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;

  console.log(`Program ID: ${program.programId.toBase58()}`);
  console.log(`Signer: ${provider.wallet.publicKey.toBase58()}`);

  // Ix params
  const decimals = 6;
  const metadata = {
    name: "Wrapped ETH",
    symbol: "wETH",
    remoteToken: toBytes(CONSTANTS.erc20Addr), // Native ETH address on Base
    scalerExponent: 12,
  };
  const gasLimit = new anchor.BN(1_000_000);

  const metadataHash = keccak256(
    Buffer.concat([
      Buffer.from(metadata.name),
      Buffer.from(metadata.symbol),
      Buffer.from(metadata.remoteToken),
      new anchor.BN(metadata.scalerExponent).toBuffer("le", 1),
    ])
  );

  const [mintPda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from(getConstantValue("wrappedTokenSeed")),
      Buffer.from([decimals]),
      toBytes(metadataHash),
    ],
    program.programId
  );

  const [bridgePda] = PublicKey.findProgramAddressSync(
    [Buffer.from(getConstantValue("bridgeSeed"))],
    program.programId
  );

  const outgoingMessage = Keypair.generate();

  console.log(`Bridge PDA: ${bridgePda.toBase58()}`);
  console.log(`Mint PDA: ${mintPda.toBase58()}`);
  console.log(`Outgoing message: ${outgoingMessage.publicKey.toBase58()}`);

  const tx = await program.methods
    .wrapToken(decimals, metadata, gasLimit)
    .accountsStrict({
      payer: provider.wallet.publicKey,
      gasFeeReceiver: getConstantValue("gasFeeReceiver"),
      mint: mintPda,
      bridge: bridgePda,
      outgoingMessage: outgoingMessage.publicKey,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    })
    .signers([outgoingMessage])
    .rpc();

  console.log("Submitted transaction:", tx);

  await confirmTransaction(provider.connection, tx);
}

main().catch((e) => {
  console.error(e);
  console.log(e.getLogs());
});
