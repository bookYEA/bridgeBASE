import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import { toBytes } from "viem";

import type { Bridge } from "../../target/types/bridge";
import { confirmTransaction } from "../utils/confirm-tx";
import { getConstantValue } from "../utils/constants";
import { ADDRESSES } from "../addresses";

type BridgeSolParams = Parameters<Program<Bridge>["methods"]["bridgeSol"]>;

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;

  // Ix params
  const gasLimit: BridgeSolParams[0] = new anchor.BN(1_000_000);
  const to: BridgeSolParams[1] = [
    ...toBytes("0x25f7fD8f50D522b266764cD3b230EDaA8CbB9f75"),
  ];
  const remoteToken: BridgeSolParams[2] = [...toBytes(ADDRESSES.wrappedSOL)];
  const amount: BridgeSolParams[3] = new anchor.BN(
    0.001 * anchor.web3.LAMPORTS_PER_SOL
  );
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

  const outgoingMessage = Keypair.generate();

  console.log(`Bridge PDA: ${bridgePda.toBase58()}`);
  console.log(`SOL Vault PDA: ${solVaultPda.toBase58()}`);
  console.log(`Outgoing message: ${outgoingMessage.publicKey.toBase58()}`);
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
      outgoingMessage: outgoingMessage.publicKey,
      systemProgram: SystemProgram.programId,
    })
    .signers([outgoingMessage])
    .rpc();

  console.log("Submitted transaction:", tx);

  await confirmTransaction(provider.connection, tx);
}

main().catch((e) => {
  console.error(e);
  // console.log(e.getLogs());
});
