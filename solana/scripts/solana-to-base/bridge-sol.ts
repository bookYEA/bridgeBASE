import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, SystemProgram, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { toBytes } from "viem";

import type { Bridge } from "../../target/types/bridge";
import { confirmTransaction } from "../utils/confirm-tx";
import { getConstantValue } from "../utils/constants";

type BridgeSolParams = Parameters<Program<Bridge>["methods"]["bridgeSol"]>;

const WRAPPED_SOL_ADDRESS = "0xfDaB33bcbD3801BE97056c3541cEC59760E23a3B";

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;

  // Ix params
  const gasLimit: BridgeSolParams[0] = new anchor.BN(1_000_000);
  const to: BridgeSolParams[1] = [
    ...toBytes("0x0000000000000000000000000000000000000000"),
  ];
  const remoteToken: BridgeSolParams[2] = [...toBytes(WRAPPED_SOL_ADDRESS)];
  const amount: BridgeSolParams[3] = new anchor.BN(1);
  const call: BridgeSolParams[4] = null;

  const [bridgePda] = PublicKey.findProgramAddressSync(
    [Buffer.from(getConstantValue("bridgeSeed"))],
    program.programId
  );

  const bridge = await program.account.bridge.fetch(bridgePda);

  const [solVaultPda] = PublicKey.findProgramAddressSync(
    [Buffer.from(getConstantValue("solVaultSeed")), Buffer.from(remoteToken)],
    program.programId
  );

  const [outgoingMessagePda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from(getConstantValue("outgoingMessageSeed")),
      bridge.nonce.toBuffer("le", 8),
    ],
    program.programId
  );

  console.log(`Bridge PDA: ${bridgePda.toBase58()}`);
  console.log(`SOL Vault PDA: ${solVaultPda.toBase58()}`);
  console.log(`Outgoing message PDA: ${outgoingMessagePda.toBase58()}`);
  console.log(`Current nonce: ${bridge.nonce.toString()}`);
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

  await confirmTransaction(provider.connection, tx);
}

main().catch((e) => {
  console.error(e);
  console.log(e.getLogs());
});
