import * as anchor from "@coral-xyz/anchor";
import { readFileSync } from "fs";
import { main as proveOnSolana } from "./solanaWithdrawal";
import { Program } from "@coral-xyz/anchor";
import type { Bridge } from "../target/types/bridge";
import type { IxParam } from "./solanaWithdrawal";
import { execSync } from "child_process";
import * as path from "path";
import { createPublicClient, decodeEventLog, http, type Hash } from "viem";
import { baseSepolia } from "viem/chains";
import { PublicKey } from "@solana/web3.js";
import { sleep } from "bun";
import { getAssociatedTokenAddress, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import baseSepoliaAddrs from "../deployments/base_sepolia.json";
import { loadFromEnv } from "./utils/loadFromEnv";

const provider = anchor.AnchorProvider.env();
anchor.setProvider(provider);

const program = anchor.workspace.Bridge as Program<Bridge>;

const VERSION = 1;

async function runBaseInteraction(): Promise<{
  nonce: number[];
  transactionHash: number[];
  remoteSender: number[];
  ixs: IxParam[];
  leafIndex: number;
  blockNumber: number;
}> {
  console.log("Executing Base interaction via Forge script...");

  const baseDir = path.resolve(__dirname, "../../base");

  const command = "make bridge-tokens-to-solana";

  let txHash: Hash;

  const publicClient = createPublicClient({
    chain: baseSepolia,
    transport: http(),
  });

  try {
    console.log(`Executing command in ${baseDir}: ${command}`);
    execSync(command, {
      cwd: baseDir,
      stdio: "pipe", // Capture stdout/stderr
      encoding: "utf-8",
    });

    const broadcastData = readFileSync(
      path.resolve(
        __dirname,
        "../../base/broadcast/BridgeTokensToSolana.s.sol/84532/run-latest.json"
      ),
      "utf-8"
    );
    const decodedData = JSON.parse(broadcastData);
    txHash = decodedData.transactions[0].hash;
  } catch (error: any) {
    console.error("Error executing Forge script:", error.message);
    if (error.stdout) {
      console.error("Forge script stdout:", error.stdout.toString());
    }
    if (error.stderr) {
      console.error("Forge script stderr:", error.stderr.toString());
    }
    throw new Error("Failed to execute Base interaction script.");
  }

  const receipt = await publicClient.getTransactionReceipt({ hash: txHash });
  const targetTopic =
    "0x157b3e0c97c86afb7b397c7ce91299a7812096cb61a249153094208e1f74a1b8";

  const targetLog = receipt.logs.find((r) => r.topics[0] === targetTopic);
  if (!targetLog) {
    throw new Error("Message passer log not found");
  }
  const decodedLog = decodeEventLog({
    abi: [
      {
        type: "event",
        name: "MessagePassed",
        inputs: [
          {
            name: "nonce",
            type: "uint256",
            indexed: true,
            internalType: "uint256",
          },
          {
            name: "sender",
            type: "address",
            indexed: true,
            internalType: "address",
          },
          {
            name: "ixs",
            type: "tuple[]",
            indexed: false,
            internalType: "struct MessagePasser.Instruction[]",
            components: [
              {
                name: "programId",
                type: "bytes32",
                internalType: "bytes32",
              },
              {
                name: "accounts",
                type: "tuple[]",
                internalType: "struct MessagePasser.AccountMeta[]",
                components: [
                  {
                    name: "pubKey",
                    type: "bytes32",
                    internalType: "bytes32",
                  },
                  {
                    name: "isSigner",
                    type: "bool",
                    internalType: "bool",
                  },
                  {
                    name: "isWritable",
                    type: "bool",
                    internalType: "bool",
                  },
                ],
              },
              {
                name: "data",
                type: "bytes",
                internalType: "bytes",
              },
            ],
          },
          {
            name: "withdrawalHash",
            type: "bytes32",
            indexed: false,
            internalType: "bytes32",
          },
        ],
        anonymous: false,
      },
    ],
    data: targetLog.data,
    topics: targetLog.topics,
  });

  const decodedIxs = decodedLog.args.ixs.map((ix) => {
    const programId = new PublicKey(Buffer.from(ix.programId.slice(2), "hex"));
    const accounts = ix.accounts.map((a) => ({
      pubkey: new PublicKey(Buffer.from(a.pubKey.slice(2), "hex")),
      ...a,
    }));
    const data = Buffer.from(ix.data.slice(2), "hex");
    return { programId, accounts, data };
  });

  console.log(`Returning data. Actual txHash from Base: ${txHash || "N/A"}`);

  return {
    nonce: Array.from(Buffer.from(targetLog.topics[1]?.slice(2) ?? "", "hex")),
    transactionHash: Array.from(
      Buffer.from(decodedLog.args.withdrawalHash.slice(2), "hex")
    ),
    remoteSender: Array.from(
      Buffer.from(decodedLog.args.sender.slice(2), "hex")
    ),
    ixs: decodedIxs,
    leafIndex: Number(BigInt(targetLog.topics[1]?.slice(6) ?? "0")),
    blockNumber: Number(targetLog.blockNumber),
  };
}

async function waitForRootOnSolana(blockNumber: number) {
  console.log("Waiting for root on Solana...");
  const [rootPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("output_root"), new anchor.BN(blockNumber).toBuffer("le", 8)],
    program.programId
  );
  let tries = 0;

  while (tries++ < 10) {
    try {
      const rootAccount = await program.account.outputRoot.fetch(rootPda);
      const blockNumberOnSolana = rootAccount.blockNumber.toNumber();

      if (blockNumberOnSolana >= blockNumber) {
        console.log("Root found on Solana. Continuing...");
        return;
      }
    } catch (e) {
      await sleep(1000);
    }
  }
}

async function finalizeTransactionOnSolana(
  transactionHash: number[],
  userATA: anchor.web3.PublicKey
) {
  const mint = new PublicKey(loadFromEnv("MINT"));
  const [vaultPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("bridge_vault"), new anchor.BN(VERSION).toBuffer("le", 1)],
    program.programId
  );
  const vaultATA = await getAssociatedTokenAddress(mint, vaultPda, true);
  const [depositPda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("deposit"),
      mint.toBuffer(),
      Buffer.from(baseSepoliaAddrs.WrappedSPL.slice(2), "hex"),
    ],
    program.programId
  );
  const remainingAccounts = [
    { pubkey: mint, isWritable: false, isSigner: false },
    { pubkey: vaultPda, isWritable: false, isSigner: false },
    { pubkey: vaultATA, isWritable: true, isSigner: false },
    { pubkey: userATA, isWritable: true, isSigner: false },
    { pubkey: TOKEN_PROGRAM_ID, isWritable: false, isSigner: false },
    { pubkey: depositPda, isWritable: true, isSigner: false },
  ];

  const tx = await program.methods
    .finalizeTransaction(transactionHash)
    .accounts({})
    .remainingAccounts(remainingAccounts)
    .rpc();

  console.log("Finalize transaction signature", tx);
  const latestBlockHash = await provider.connection.getLatestBlockhash();
  await provider.connection.confirmTransaction(
    {
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: tx,
    },
    "confirmed"
  );

  const [messagePda] = PublicKey.findProgramAddressSync(
    [Buffer.from("message"), Buffer.from(transactionHash)],
    program.programId
  );
  const message = await program.account.message.fetch(messagePda);
  if (!message.successfulMessage) {
    throw new Error("Finalize transaction completed but message failed");
  }
}

async function orchestrate() {
  try {
    console.log("Starting bridge orchestration...");

    // 1. Run the Base-side interaction and get parameters
    const {
      nonce,
      transactionHash,
      remoteSender,
      ixs,
      leafIndex,
      blockNumber,
    } = await runBaseInteraction();

    console.log("Received parameters from Base interaction:", {
      nonce,
      transactionHash,
      remoteSender,
      ixs,
      blockNumber,
    });

    await waitForRootOnSolana(blockNumber);

    // 2. Prove the transaction on Solana using the parameters from Base
    console.log("Initiating Solana transaction proof...");
    await proveOnSolana(
      nonce,
      transactionHash,
      remoteSender,
      ixs,
      leafIndex,
      blockNumber
    );

    console.log("Solana transaction proven successfully.");

    // TODO: extract this from ixs
    const userATA = new PublicKey(
      Buffer.from(
        "1e1112994ab6232a643fd9d1cff130f5b829ac336b910b91c6304a96f776fd9c",
        "hex"
      )
    );

    // 3. Finalize the transaction on Solana
    console.log("Finalizing transaction on Solana...");
    await finalizeTransactionOnSolana(transactionHash, userATA);

    console.log("Solana transaction finalized successfully.");

    console.log("Bridge orchestration completed.");
  } catch (error) {
    console.error("Error during bridge orchestration:", error);
    if (
      error &&
      typeof error === "object" &&
      "getLogs" in error &&
      typeof error.getLogs === "function"
    ) {
      console.error("Transaction logs:", error.getLogs());
    }
    process.exit(1);
  }
}

orchestrate();
