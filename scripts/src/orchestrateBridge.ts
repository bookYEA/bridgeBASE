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
import { SYSTEM_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/native/system";
import MessagePassedEvent from "./abis/MessagePassedEvent";

const IS_SOL = loadFromEnv("IS_SOL", true) === "true";
const IS_ERC20 = loadFromEnv("IS_ERC20", true) === "true";

const provider = anchor.AnchorProvider.env();
anchor.setProvider(provider);

const program = anchor.workspace.Bridge as Program<Bridge>;

const VERSION = 2;
const solLocalAddress = new PublicKey(
  Buffer.from(
    "0501550155015501550155015501550155015501550155015501550155015501",
    "hex"
  )
);

function getBaseContractsCommand(): string {
  if (IS_SOL) {
    return "make bridge-sol-to-solana";
  } else if (IS_ERC20) {
    return "make bridge-erc20-to-solana";
  }
  return "make bridge-tokens-to-solana";
}

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
  const command = getBaseContractsCommand();

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
    const transaction = decodedData.transactions.find(
      (t: any) =>
        t.contractAddress.toLowerCase() ===
        baseSepoliaAddrs.Bridge.toLowerCase()
    );
    if (!transaction) {
      throw new Error("Bridge transaction not found in receipt");
    }
    txHash = transaction.hash;
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
    abi: MessagePassedEvent,
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
    leafIndex: Number(BigInt("0x" + (targetLog.topics[1]?.slice(6) ?? "0"))),
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

async function getRemainingAccounts(recipient: PublicKey): Promise<
  {
    pubkey: PublicKey;
    isWritable: boolean;
    isSigner: boolean;
  }[]
> {
  if (IS_SOL) {
    const [depositPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("deposit"),
        solLocalAddress.toBuffer(),
        Buffer.from(baseSepoliaAddrs.WrappedSOL.slice(2), "hex"),
      ],
      program.programId
    );
    return [
      { pubkey: recipient, isWritable: true, isSigner: false },
      { pubkey: SYSTEM_PROGRAM_ID, isWritable: false, isSigner: false },
      { pubkey: depositPda, isWritable: true, isSigner: false },
    ];
  } else if (IS_ERC20) {
    const mint = new PublicKey(loadFromEnv(IS_ERC20 ? "ERC20_MINT" : "MINT"));
    return [
      { pubkey: recipient, isWritable: true, isSigner: false },
      { pubkey: mint, isWritable: true, isSigner: false },
      { pubkey: TOKEN_PROGRAM_ID, isWritable: false, isSigner: false },
    ];
  }

  const mint = new PublicKey(loadFromEnv(IS_ERC20 ? "ERC20_MINT" : "MINT"));
  const remoteToken = IS_ERC20
    ? baseSepoliaAddrs.ERC20
    : baseSepoliaAddrs.WrappedSPL;
  const [vaultPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("bridge_vault"), new anchor.BN(VERSION).toBuffer("le", 1)],
    program.programId
  );
  const vaultATA = await getAssociatedTokenAddress(mint, vaultPda, true);
  const [depositPda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("deposit"),
      mint.toBuffer(),
      Buffer.from(remoteToken.slice(2), "hex"),
    ],
    program.programId
  );
  return [
    { pubkey: mint, isWritable: false, isSigner: false },
    { pubkey: vaultPda, isWritable: false, isSigner: false },
    { pubkey: vaultATA, isWritable: true, isSigner: false },
    { pubkey: recipient, isWritable: true, isSigner: false },
    { pubkey: TOKEN_PROGRAM_ID, isWritable: false, isSigner: false },
    { pubkey: depositPda, isWritable: true, isSigner: false },
  ];
}

async function finalizeTransactionOnSolana(
  transactionHash: number[],
  recipient: anchor.web3.PublicKey
) {
  const remainingAccounts = await getRemainingAccounts(recipient);

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

// NOTE: This assumes there is one relevant bridge ix and that it includes no accounts
function extractRecipientFromIxs(ixs: number[]): PublicKey {
  if (ixs.length < 204) {
    throw new Error("Ixs length unexpectedly short");
  }
  const recipient = ixs.slice(172, 204);
  return new PublicKey(Buffer.from(recipient));
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

    // 3. Finalize the transaction on Solana
    console.log("Finalizing transaction on Solana...");
    await finalizeTransactionOnSolana(
      transactionHash,
      extractRecipientFromIxs(ixs[0].data)
    );

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
