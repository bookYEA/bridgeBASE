import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { toBytes } from "viem";

import type { Bridge } from "../../target/types/bridge";
import { confirmTransaction } from "../utils/confirm-tx";
import { getConstantValue } from "../utils/constants";
import { CONSTANTS } from "../constants";

type BridgeCallParams = Parameters<Program<Bridge>["methods"]["bridgeCall"]>;

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;

  // Ix parameters
  const gasLimit: BridgeCallParams[0] = new anchor.BN(1000000);
  const call: BridgeCallParams[1] = {
    ty: { call: {} },
    to: toBytes(CONSTANTS.counterAddress),
    value: new anchor.BN(0),
    data: Buffer.from("d09de08a", "hex"), // increment()
  };

  const [bridgePda] = PublicKey.findProgramAddressSync(
    [Buffer.from(getConstantValue("bridgeSeed"))],
    program.programId
  );

  const bridge = await program.account.bridge.fetch(bridgePda);

  const outgoingMessage = Keypair.generate();

  console.log(`Bridge PDA: ${bridgePda.toBase58()}`);
  console.log(`Outgoing message: ${outgoingMessage.publicKey.toBase58()}`);
  console.log(`Current nonce: ${bridge.nonce.toString()}`);

  const tx = await program.methods
    .bridgeCall(gasLimit, call)
    .accountsStrict({
      payer: provider.wallet.publicKey,
      from: provider.wallet.publicKey,
      gasFeeReceiver: getConstantValue("gasFeeReceiver"),
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
  console.log(e.getLogs());
});
