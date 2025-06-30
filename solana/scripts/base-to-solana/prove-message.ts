import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { createPublicClient, http, toBytes, toHex } from "viem";
import { baseSepolia } from "viem/chains";
import { decodeEventLog } from "viem/utils";

import type { Bridge } from "../../target/types/bridge";
import { BRIDGE_ABI } from "../utils/bridge.abi";
import { getConstantValue } from "../utils/constants";
import { ADDRESSES } from "../addresses";
import { confirmTransaction } from "../utils/confirm-tx";

const TRANSACTION_HASH =
  "0x1558d0b5366a42738af3952ff985ed27e604a405793dec8476a38ff2cd3100b8";

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;

  const [bridgePda] = PublicKey.findProgramAddressSync(
    [Buffer.from(getConstantValue("bridgeSeed"))],
    program.programId
  );

  const bridge = await program.account.bridge.fetch(bridgePda);

  const [outputRootPda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from(getConstantValue("outputRootSeed")),
      bridge.baseBlockNumber.toBuffer("le", 8),
    ],
    program.programId
  );

  const outputRoot = await program.account.outputRoot.fetch(outputRootPda);
  console.log(`Output Root: ${toHex(Uint8Array.from(outputRoot.root))}`);

  const { event, rawProof, leafIndex, totalLeafCount } = await generateProof(
    TRANSACTION_HASH,
    bridge.baseBlockNumber
  );

  const proof = {
    proof: rawProof.map((e) => [...toBytes(e)]),
    leafIndex: new anchor.BN(Number(leafIndex)),
    totalLeafCount: new anchor.BN(Number(totalLeafCount)),
  };

  // Ix params
  const nonce = new anchor.BN(event.message.nonce);
  const sender = [...toBytes(event.message.sender)];
  const data = Buffer.from(toBytes(event.message.data));
  const messageHash = [...toBytes(event.messageHash)];

  const [messagePda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from(getConstantValue("incomingMessageSeed")),
      toBytes(event.messageHash),
    ],
    program.programId
  );

  console.log(`Message PDA: ${messagePda.toBase58()}`);
  console.log(`Output Root PDA: ${outputRootPda.toBase58()}`);
  console.log(`Nonce: ${nonce.toString()}`);
  console.log(`Sender: ${event.message.sender}`);
  console.log(`Data: ${event.message.data}`);
  console.log(`Message Hash: ${event.messageHash}`);
  console.log(`Proof: ${JSON.stringify(proof)}`);

  const tx = await program.methods
    .proveMessage(nonce, sender, data, proof, messageHash)
    .accountsStrict({
      payer: provider.wallet.publicKey,
      outputRoot: outputRootPda,
      message: messagePda,
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

async function generateProof(
  transactionHash: `0x${string}`,
  bridgeBaseBlockNumber: bigint
) {
  const publicClient = createPublicClient({
    chain: baseSepolia,
    transport: http(),
  });

  const txReceipt = await publicClient.getTransactionReceipt({
    hash: transactionHash,
  });

  // Extract and decode MessageRegistered events
  const messageRegisteredEvents = txReceipt.logs
    .map((log) => {
      try {
        const decodedLog = decodeEventLog({
          abi: BRIDGE_ABI,
          data: log.data,
          topics: log.topics,
        });

        return decodedLog.eventName === "MessageRegistered"
          ? {
              messageHash: decodedLog.args.messageHash,
              mmrRoot: decodedLog.args.mmrRoot,
              message: decodedLog.args.message,
            }
          : null;
      } catch (error) {
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

  const event = messageRegisteredEvents[0]!;

  console.log("\n=== MessageRegistered Event ===");
  console.log("Message Hash:", event.messageHash);
  console.log("MMR Root:", event.mmrRoot);
  console.log("Nonce:", event.message.nonce);
  console.log("Sender:", event.message.sender);
  console.log("Data:", event.message.data);

  // FIXME: Something is off here, the returned proof does not match the MMR root registered in the OutputRoot PDA
  //        even though we're using the same block number
  const [rawProof, totalLeafCount] = await publicClient.readContract({
    address: ADDRESSES.bridge,
    abi: BRIDGE_ABI,
    functionName: "generateProof",
    args: [event.message.nonce],
    blockNumber: bridgeBaseBlockNumber,
  });

  console.log("\n=== Bridge Contract Proof ===");
  console.log(
    `Proof at block ${bridgeBaseBlockNumber.toString()}: ${rawProof} (index: ${
      event.message.nonce
    } / total: ${totalLeafCount})`
  );

  return {
    event,
    rawProof,
    leafIndex: event.message.nonce,
    totalLeafCount,
  };
}
