import * as anchor from "@coral-xyz/anchor";
import { readFileSync } from "fs";
import { Program } from "@coral-xyz/anchor";
import type { Bridge } from "../target/types/bridge";
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

type IxParam = Parameters<
  Program<Bridge>["methods"]["proveTransaction"]
>[3][number];
type Account = { pubkey: PublicKey; isWritable: boolean; isSigner: boolean };

const IS_SOL = loadFromEnv("IS_SOL", true) === "true";
const IS_ERC20 = loadFromEnv("IS_ERC20", true) === "true";
const IS_ETH = loadFromEnv("IS_ETH", true) === "true";

let CFG: {
  provider: anchor.AnchorProvider;
  program: Program<Bridge>;
  version: anchor.BN;
  localToken: PublicKey;
  remoteToken: Buffer;
  baseContractsCommand: string;
  remainingAccounts: Account[];
  messagePassedEventTopic: string;
  oracleUrl: string;
};

async function init() {
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.Bridge as Program<Bridge>;

  let localToken: PublicKey;
  let remoteToken: Buffer;
  let remainingAccounts: Account[];
  let baseContractsCommand: string;

  if (IS_SOL) {
    localToken = new PublicKey(
      Buffer.from(
        "0501550155015501550155015501550155015501550155015501550155015501",
        "hex"
      )
    );
    remoteToken = Buffer.from(baseSepoliaAddrs.WrappedSOL.slice(2), "hex");
    baseContractsCommand = "make bridge-sol-to-solana";
  } else if (IS_ERC20) {
    localToken = new PublicKey(loadFromEnv("ERC20_MINT"));
    remoteToken = Buffer.from(baseSepoliaAddrs.ERC20.slice(2), "hex");
    baseContractsCommand = "make bridge-erc20-to-solana";
  } else if (IS_ETH) {
    localToken = new PublicKey(loadFromEnv("ETH_MINT"));
    remoteToken = Buffer.from(baseSepoliaAddrs.ETH.slice(2), "hex");
    baseContractsCommand = "make bridge-eth-to-solana";
  } else {
    localToken = new PublicKey(loadFromEnv("MINT"));
    remoteToken = Buffer.from(baseSepoliaAddrs.WrappedSPL.slice(2), "hex");
    baseContractsCommand = "make bridge-tokens-to-solana";
  }

  const [depositPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("deposit"), localToken.toBuffer(), remoteToken],
    program.programId
  );

  const version = new anchor.BN(2);

  if (IS_SOL) {
    remainingAccounts = [
      { pubkey: SYSTEM_PROGRAM_ID, isWritable: false, isSigner: false },
      { pubkey: depositPda, isWritable: true, isSigner: false },
    ];
  } else if (IS_ERC20 || IS_ETH) {
    remainingAccounts = [
      { pubkey: localToken, isWritable: true, isSigner: false },
      { pubkey: TOKEN_PROGRAM_ID, isWritable: false, isSigner: false },
    ];
  } else {
    const [vaultPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("bridge_vault"), version.toBuffer("le", 1)],
      program.programId
    );
    const vaultATA = await getAssociatedTokenAddress(
      localToken,
      vaultPda,
      true
    );
    remainingAccounts = [
      { pubkey: localToken, isWritable: false, isSigner: false },
      { pubkey: vaultPda, isWritable: false, isSigner: false },
      { pubkey: vaultATA, isWritable: true, isSigner: false },
      { pubkey: TOKEN_PROGRAM_ID, isWritable: false, isSigner: false },
      { pubkey: depositPda, isWritable: true, isSigner: false },
    ];
  }

  CFG = {
    provider: anchor.AnchorProvider.env(),
    program: program,
    version: version,
    localToken: localToken,
    remoteToken: remoteToken,
    baseContractsCommand: baseContractsCommand,
    remainingAccounts: remainingAccounts,
    messagePassedEventTopic:
      "0x157b3e0c97c86afb7b397c7ce91299a7812096cb61a249153094208e1f74a1b8",
    oracleUrl: "http://localhost:8080",
  };
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
  const command = CFG.baseContractsCommand;

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

  const targetLog = receipt.logs.find(
    (r) => r.topics[0] === CFG.messagePassedEventTopic
  );
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

function getRootPda(blockNumber: number): PublicKey {
  const [rootPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("output_root"), new anchor.BN(blockNumber).toBuffer("le", 8)],
    CFG.program.programId
  );
  return rootPda;
}

async function waitForRootOnSolana(blockNumber: number) {
  console.log("Waiting for root on Solana...");
  const rootPda = getRootPda(blockNumber);
  let tries = 0;

  while (tries++ < 10) {
    try {
      const rootAccount = await CFG.program.account.outputRoot.fetch(rootPda);
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

async function confirmTransaction(tx: string) {
  const latestBlockHash = await CFG.provider.connection.getLatestBlockhash();
  await CFG.provider.connection.confirmTransaction(
    {
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: tx,
    },
    "confirmed"
  );
}

async function proveOnSolana(
  nonce: number[],
  transactionHash: number[],
  remoteSender: number[],
  ixs: IxParam[],
  leafIndex: number,
  blockNumber: number
) {
  const oracleUrl = `${CFG.oracleUrl}/proof/${leafIndex}`;
  const res = await fetch(oracleUrl);
  const json = await res.json();

  const rootPda = getRootPda(blockNumber);

  const tx = await CFG.program.methods
    .proveTransaction(
      transactionHash,
      nonce,
      remoteSender,
      ixs,
      json.proof.map((element: string) =>
        Array.from(Buffer.from(element, "base64"))
      ),
      new anchor.BN(leafIndex),
      new anchor.BN(json.totalLeafCount)
    )
    .accounts({ root: rootPda })
    .rpc();

  console.log("Prove transaction signature", tx);
  await confirmTransaction(tx);
}

async function finalizeTransactionOnSolana(
  transactionHash: number[],
  recipient: anchor.web3.PublicKey
) {
  const remainingAccounts = [
    ...CFG.remainingAccounts,
    { pubkey: recipient, isWritable: true, isSigner: false },
  ];

  const tx = await CFG.program.methods
    .finalizeTransaction(transactionHash)
    .accounts({})
    .remainingAccounts(remainingAccounts)
    .rpc();

  console.log("Finalize transaction signature", tx);
  await confirmTransaction(tx);

  const [messagePda] = PublicKey.findProgramAddressSync(
    [Buffer.from("message"), Buffer.from(transactionHash)],
    CFG.program.programId
  );
  const message = await CFG.program.account.message.fetch(messagePda);
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
  await init();
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
