import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  getAssociatedTokenAddressSync,
  TOKEN_2022_PROGRAM_ID,
} from "@solana/spl-token";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { toBytes } from "viem";

import type { Bridge } from "../../target/types/bridge";
import { getConstantValue } from "../utils/constants";
import { confirmTransaction } from "../utils/confirm-tx";
import { CONSTANTS } from "../constants";

type BridgeWrappedTokenParams = Parameters<
  Program<Bridge>["methods"]["bridgeWrappedToken"]
>;

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;

  console.log(`Program ID: ${program.programId.toBase58()}`);
  console.log(`Sender: ${provider.wallet.publicKey.toBase58()}`);

  // Ix params
  const gasLimit: BridgeWrappedTokenParams[0] = new anchor.BN(1_000_000);
  const to: BridgeWrappedTokenParams[1] = toBytes(CONSTANTS.recipient);
  const amount: BridgeWrappedTokenParams[2] = new anchor.BN(1);
  const call: BridgeWrappedTokenParams[3] = null;

  const [bridgePda] = PublicKey.findProgramAddressSync(
    [Buffer.from(getConstantValue("bridgeSeed"))],
    program.programId
  );

  const bridge = await program.account.bridge.fetch(bridgePda);

  const [outgoingMessagePda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from(getConstantValue("outgoingMessageSeed")),
      bridge.nonce.toBuffer("le", 8),
    ],
    program.programId
  );

  // Get user's token account
  const mint = new PublicKey(CONSTANTS.wrappedERC20);
  const fromTokenAccount = getAssociatedTokenAddressSync(
    mint,
    provider.wallet.publicKey,
    false,
    TOKEN_2022_PROGRAM_ID
  );

  console.log(`Bridge PDA: ${bridgePda.toBase58()}`);
  console.log(`Outgoing message PDA: ${outgoingMessagePda.toBase58()}`);
  console.log(`From token account: ${fromTokenAccount.toBase58()}`);
  console.log(`Current nonce: ${bridge.nonce.toString()}`);
  console.log(`Bridging amount: ${amount.toNumber()}`);

  const tx = await program.methods
    .bridgeWrappedToken(gasLimit, to, amount, call)
    .accountsStrict({
      payer: provider.wallet.publicKey,
      from: provider.wallet.publicKey,
      gasFeeReceiver: getConstantValue("gasFeeReceiver"),
      mint: mint,
      fromTokenAccount: fromTokenAccount,
      bridge: bridgePda,
      outgoingMessage: outgoingMessagePda,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    })
    .rpc();

  console.log("Submitted transaction:", tx);

  await confirmTransaction(provider.connection, tx);
}

main().catch((e) => {
  console.error(e);
  console.log(e.getLogs());
});
