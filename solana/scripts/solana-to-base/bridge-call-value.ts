import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  getAssociatedTokenAddressSync,
  TOKEN_2022_PROGRAM_ID,
} from "@solana/spl-token";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { createPublicClient, http, toBytes } from "viem";
import { baseSepolia } from "viem/chains";

import type { Bridge } from "../../target/types/bridge";
import { BRIDGE_ABI } from "../utils/bridge.abi";
import { confirmTransaction } from "../utils/confirm-tx";
import { getConstantValue } from "../utils/constants";
import { ADDRESSES } from "../addresses";
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

  const publicClient = createPublicClient({
    chain: baseSepolia,
    transport: http(),
  });

  const twinAddress = await publicClient.readContract({
    address: ADDRESSES.bridge,
    abi: BRIDGE_ABI,
    functionName: "getPredictedTwinAddress",
    args: [`0x${provider.wallet.publicKey.toBuffer().toString("hex")}`],
  });

  console.log(`Twin address: ${twinAddress}`);

  // Ix params
  const gasLimit: BridgeWrappedTokenParams[0] = new anchor.BN(1_000_000);
  const to: BridgeWrappedTokenParams[1] = toBytes(twinAddress);
  const amount: BridgeWrappedTokenParams[2] = new anchor.BN(1);
  const call: BridgeWrappedTokenParams[3] = {
    ty: { call: {} }, // Call
    to: toBytes(CONSTANTS.counterValue),
    value: new anchor.BN(1000000000000),
    data: Buffer.from(toBytes("0xd09de08a")), // increment()
  };

  const [bridgePda] = PublicKey.findProgramAddressSync(
    [Buffer.from(getConstantValue("bridgeSeed"))],
    program.programId
  );

  const bridge = await program.account.bridge.fetch(bridgePda);

  const outgoingMessage = Keypair.generate();

  // Get user's token account
  const mint = new PublicKey(CONSTANTS.wrappedEth);
  const fromTokenAccount = getAssociatedTokenAddressSync(
    mint,
    provider.wallet.publicKey,
    false,
    TOKEN_2022_PROGRAM_ID
  );

  console.log(`Bridge PDA: ${bridgePda.toBase58()}`);
  console.log(`Outgoing message: ${outgoingMessage.publicKey.toBase58()}`);
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
