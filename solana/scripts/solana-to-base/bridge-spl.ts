import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  getAssociatedTokenAddressSync,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { toBytes } from "viem";

import type { Bridge } from "../../target/types/bridge";
import { confirmTransaction } from "../utils/confirm-tx";
import { getConstantValue } from "../utils/constants";
import { ADDRESSES } from "../addresses";
import { CONSTANTS } from "../constants";

type BridgeSplParams = Parameters<Program<Bridge>["methods"]["bridgeSpl"]>;

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;

  console.log(`Program ID: ${program.programId.toBase58()}`);
  console.log(`Signer: ${provider.wallet.publicKey.toBase58()}`);

  // Ix params
  const gasLimit: BridgeSplParams[0] = new anchor.BN(1_000_000);
  const to: BridgeSplParams[1] = toBytes(CONSTANTS.recipient);
  const remoteToken: BridgeSplParams[2] = toBytes(ADDRESSES.wrappedSPL);
  const amount: BridgeSplParams[3] = new anchor.BN(1);
  const call: BridgeSplParams[4] = null;

  const [bridgePda] = PublicKey.findProgramAddressSync(
    [Buffer.from(getConstantValue("bridgeSeed"))],
    program.programId
  );

  const bridge = await program.account.bridge.fetch(bridgePda);

  const mint = new PublicKey(CONSTANTS.solanaSpl);
  const [tokenVaultPda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from(getConstantValue("tokenVaultSeed")),
      mint.toBuffer(),
      Buffer.from(remoteToken),
    ],
    program.programId
  );

  const outgoingMessage = Keypair.generate();

  const fromTokenAccount = getAssociatedTokenAddressSync(
    mint,
    provider.wallet.publicKey,
    false,
    TOKEN_PROGRAM_ID
  );

  console.log(`Bridge PDA: ${bridgePda.toBase58()}`);
  console.log(`Token Vault PDA: ${tokenVaultPda.toBase58()}`);
  console.log(`Outgoing message: ${outgoingMessage.publicKey.toBase58()}`);
  console.log(`From token account: ${fromTokenAccount.toBase58()}`);
  console.log(`Current nonce: ${bridge.nonce.toString()}`);
  console.log(`Bridging amount: ${amount.toNumber()}`);

  const tx = await program.methods
    .bridgeSpl(gasLimit, to, remoteToken, amount, call)
    .accountsStrict({
      payer: provider.wallet.publicKey,
      from: provider.wallet.publicKey,
      gasFeeReceiver: getConstantValue("gasFeeReceiver"),
      mint: mint,
      fromTokenAccount: fromTokenAccount,
      tokenVault: tokenVaultPda,
      bridge: bridgePda,
      outgoingMessage: outgoingMessage.publicKey,
      tokenProgram: TOKEN_PROGRAM_ID,
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
