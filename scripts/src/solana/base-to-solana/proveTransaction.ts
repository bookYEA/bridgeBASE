import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";

import type { Bridge } from "../../../target/types/bridge";
import { confirmTransaction } from "../../utils/confirmTransaction";
import { SYSTEM_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/native/system";
import {
  createPublicClient,
  http,
  decodeEventLog,
  type Address,
  toBytes,
  type Hex,
} from "viem";
import { baseSepolia } from "viem/chains";
import BridgeAbi from "../../../abis/Bridge.json";
import baseSepoliaAddrs from "../../../deployments/base_sepolia.json";

const TRANSACTION_HASH =
  "0x1558d0b5366a42738af3952ff985ed27e604a405793dec8476a38ff2cd3100b8";

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;

  const publicClient = createPublicClient({
    chain: baseSepolia,
    transport: http(),
  });
  const baseTransaction = await publicClient.getTransactionReceipt({
    hash: TRANSACTION_HASH,
  });

  const { logs } = baseTransaction;

  // Extract and decode MessageRegistered events
  const messageRegisteredEvents = logs
    .map((log) => {
      try {
        const decodedLog = decodeEventLog({
          abi: BridgeAbi,
          data: log.data,
          topics: log.topics,
        });

        // Filter for MessageRegistered events
        if (decodedLog.eventName === "MessageRegistered") {
          const args = decodedLog.args as unknown as {
            messageHash: `0x${string}`;
            mmrRoot: `0x${string}`;
            message: {
              nonce: bigint;
              sender: `0x${string}`;
              data: `0x${string}`;
            };
          };

          return {
            messageHash: args.messageHash,
            mmrRoot: args.mmrRoot,
            message: args.message,
            logIndex: log.logIndex,
            transactionIndex: log.transactionIndex,
          };
        }
        return null;
      } catch (error) {
        // Skip logs that don't match our ABI
        return null;
      }
    })
    .filter((event) => event !== null);

  console.log("\n=== MessageRegistered Events ===");
  console.log(
    `Found ${messageRegisteredEvents.length} MessageRegistered event(s):`
  );

  if (messageRegisteredEvents.length !== 1) {
    throw new Error("Unexpected number of MessageRegistered events detected");
  }

  const event = messageRegisteredEvents[0];

  console.log("\n=== First MessageRegistered Event Details ===");
  console.log("Message Hash:", event.messageHash);
  console.log("MMR Root:", event.mmrRoot);
  console.log("Nonce:", event.message.nonce);
  console.log("Sender:", event.message.sender);
  console.log("Data:", event.message.data);

  const bridgePda = PublicKey.findProgramAddressSync(
    [Buffer.from("bridge")],
    program.programId
  )[0];

  const bridgeAccount = await program.account.bridge.fetch(bridgePda);

  // Query root from bridge contract at the block number
  const rawProof: [Hex[], number] = (await publicClient.readContract({
    address: baseSepoliaAddrs.Bridge as Address,
    abi: BridgeAbi,
    functionName: "generateProof",
    args: [event.message.nonce],
    blockNumber: bridgeAccount.baseBlockNumber,
  })) as unknown as [Hex[], number];

  console.log("\n=== Bridge Contract Proof ===");
  console.log(
    "Proof at block",
    bridgeAccount.baseBlockNumber.toString() + ":",
    rawProof
  );

  const outputRootPda = PublicKey.findProgramAddressSync(
    [
      Buffer.from("output_root"),
      bridgeAccount.baseBlockNumber.toBuffer("le", 8),
    ],
    program.programId
  )[0];
  const messagePda = PublicKey.findProgramAddressSync(
    [Buffer.from("incoming_message"), toBytes(event.messageHash)],
    program.programId
  )[0];

  const proof = {
    proof: rawProof[0].map((el) => toBytes(el)),
    leafIndex: new anchor.BN(event.message.nonce),
    totalLeafCount: new anchor.BN(rawProof[1]),
  };

  console.log({
    nonce: new anchor.BN(event.message.nonce),
    sender: Array.from(toBytes(event.message.sender)),
    data: Buffer.from(toBytes(event.message.data)),
    proof,
  });

  console.log({
    payer: provider.wallet.publicKey.toBase58(),
    messagePda: messagePda.toBase58(),
  });

  const tx = await program.methods
    .proveMessage(
      new anchor.BN(event.message.nonce),
      Array.from(toBytes(event.message.sender)),
      Buffer.from(toBytes(event.message.data)),
      proof,
      Array.from(toBytes(event.messageHash))
    )
    .accountsStrict({
      payer: provider.wallet.publicKey,
      outputRoot: outputRootPda,
      message: messagePda,
      systemProgram: SYSTEM_PROGRAM_ID,
    })
    .rpc();

  console.log("Submitted transaction:", tx);

  await confirmTransaction(provider.connection, tx);
}

main().catch((error) => {
  console.error("Error during bridge orchestration:", error);
  if (
    error &&
    typeof error === "object" &&
    "getLogs" in error &&
    typeof error.getLogs === "function"
  ) {
    console.error("Transaction logs:", error.getLogs());
  }
});
