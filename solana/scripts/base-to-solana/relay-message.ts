import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  TOKEN_2022_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  type Account,
} from "@solana/spl-token";
import { toBytes } from "viem";
import { PublicKey } from "@solana/web3.js";

import type { Bridge } from "../../target/types/bridge";
import { getConstantValue } from "../utils/constants";
import { confirmTransaction } from "../utils/confirm-tx";
import { deserializeMessage } from "../utils/deserializer";
import { SYSTEM_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/native/system";

// The message hash from a previously proven message
const MESSAGE_HASH =
  "0xbde92c4328e72e69a66b741c0aa5f7082c59446cf7dca097f3b3249f6e7d3d87";

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;

  // Find the message PDA using the message hash (from prove-message)
  const [messagePda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from(getConstantValue("incomingMessageSeed")),
      toBytes(MESSAGE_HASH),
    ],
    program.programId
  );

  // Fetch the message to get the sender for the bridge CPI authority
  const message = await program.account.incomingMessage.fetch(messagePda);

  // Find the bridge CPI authority PDA
  const [bridgeCpiAuthorityPda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from(getConstantValue("bridgeCpiAuthoritySeed")),
      Buffer.from(message.sender),
    ],
    program.programId
  );

  console.log(`Message PDA: ${messagePda.toBase58()}`);
  console.log(`Bridge CPI Authority PDA: ${bridgeCpiAuthorityPda.toBase58()}`);
  console.log(`Message executed: ${message.executed}`);

  if (message.executed) {
    console.log("Message has already been executed!");
    return;
  }

  const messageData = Buffer.from(message.data);
  const deserializedMessage = deserializeMessage(messageData);

  if (deserializedMessage.type === "Call") {
    console.log(
      `Call message with ${deserializedMessage.ixs.length} instructions`
    );
  } else if (deserializedMessage.type === "Transfer") {
    console.log(
      `Transfer message with ${deserializedMessage.ixs.length} instructions`
    );

    const requiredAccounts = {
      payer: provider.wallet.publicKey,
      bridgeCpiAuthority: bridgeCpiAuthorityPda,
      message: messagePda,
    };
    let remainingAccounts: {
      pubkey: anchor.web3.PublicKey;
      isWritable: boolean;
      isSigner: boolean;
    }[];

    if (deserializedMessage.transfer.type === "Sol") {
      console.log("SOL transfer detected");
      const solTransfer = deserializedMessage.transfer;

      console.log(`SOL transfer:`);
      console.log(
        `  Remote token: 0x${solTransfer.remoteToken.toString("hex")}`
      );
      console.log(`  To: ${solTransfer.to.toBase58()}`);
      console.log(`  Amount: ${solTransfer.amount}`);

      const [solVaultPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from(getConstantValue("solVaultSeed")),
          Buffer.from(solTransfer.remoteToken),
        ],
        program.programId
      );

      remainingAccounts = [
        {
          pubkey: solVaultPda,
          isWritable: true,
          isSigner: false,
        },
        {
          pubkey: solTransfer.to,
          isWritable: true,
          isSigner: false,
        },
        {
          pubkey: SYSTEM_PROGRAM_ID,
          isWritable: false,
          isSigner: false,
        },
      ];
    } else if (deserializedMessage.transfer.type === "Spl") {
      console.log("SPL transfer detected");
      const splTransfer = deserializedMessage.transfer;

      console.log(`SPL transfer:`);
      console.log(
        `  RemoteToken: 0x${splTransfer.remoteToken.toString("hex")}`
      );
      console.log(`  LocalToken: ${splTransfer.localToken.toBase58()}`);
      console.log(`  To: ${splTransfer.to.toBase58()}`);
      console.log(`  Amount: ${splTransfer.amount}`);

      const [tokenVaultPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from(getConstantValue("tokenVaultSeed")),
          splTransfer.localToken.toBuffer(),
          Buffer.from(splTransfer.remoteToken),
        ],
        program.programId
      );

      remainingAccounts = [
        {
          pubkey: splTransfer.localToken,
          isWritable: false,
          isSigner: false,
        },
        {
          pubkey: tokenVaultPda,
          isWritable: true,
          isSigner: false,
        },
        {
          pubkey: splTransfer.to,
          isWritable: true,
          isSigner: false,
        },
        {
          pubkey: TOKEN_PROGRAM_ID,
          isWritable: false,
          isSigner: false,
        },
        {
          pubkey: TOKEN_2022_PROGRAM_ID,
          isWritable: false,
          isSigner: false,
        },
      ];
    } else if (deserializedMessage.transfer.type === "WrappedToken") {
      const wrappedTransfer = deserializedMessage.transfer;

      console.log(`WrappedToken transfer:`);
      console.log(`  Local Token: ${wrappedTransfer.localToken.toBase58()}`);
      console.log(`  To: ${wrappedTransfer.to.toBase58()}`);
      console.log(`  Amount: ${wrappedTransfer.amount}`);

      remainingAccounts = [
        {
          pubkey: wrappedTransfer.localToken,
          isWritable: true,
          isSigner: false,
        },
        {
          pubkey: wrappedTransfer.to,
          isWritable: true,
          isSigner: false,
        },
        {
          pubkey: TOKEN_2022_PROGRAM_ID,
          isWritable: false,
          isSigner: false,
        },
      ];
    } else {
      throw new Error("Unexpected transfer type detected");
    }

    const tx = await program.methods
      .relayMessage()
      .accountsStrict(requiredAccounts)
      .remainingAccounts(remainingAccounts)
      .rpc();

    console.log("Submitted transaction:", tx);

    await confirmTransaction(provider.connection, tx);
  }
}

main().catch((e) => {
  console.error(e);
  console.log(e.getLogs());
});
