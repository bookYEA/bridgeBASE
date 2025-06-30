import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { TOKEN_2022_PROGRAM_ID } from "@solana/spl-token";
import { toBytes } from "viem";
import { PublicKey } from "@solana/web3.js";

import type { Bridge } from "../../target/types/bridge";
import { getConstantValue } from "../utils/constants";
import { confirmTransaction } from "../utils/confirm-tx";
import { deserializeMessage } from "../utils/deserializer";

// The message hash from a previously proven message
const MESSAGE_HASH =
  "0x51dfebda7e0f66c2f26c91ce3af466fe348291864cbeec2c5eef4bea3badd74c";

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

    if (deserializedMessage.transfer.type === "Sol") {
      console.log("SOL transfer detected");
    } else if (deserializedMessage.transfer.type === "Spl") {
      console.log("SPL transfer detected");
    } else if (deserializedMessage.transfer.type === "WrappedToken") {
      const wrappedTransfer = deserializedMessage.transfer;

      console.log(`WrappedToken transfer:`);
      console.log(`  Local Token: ${wrappedTransfer.localToken.toBase58()}`);
      console.log(`  To: ${wrappedTransfer.to.toBase58()}`);
      console.log(`  Amount: ${wrappedTransfer.amount}`);

      const tx = await program.methods
        .relayMessage()
        .accountsStrict({
          payer: provider.wallet.publicKey,
          bridgeCpiAuthority: bridgeCpiAuthorityPda,
          message: messagePda,
        })
        .remainingAccounts([
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
        ])
        .rpc();

      console.log("Submitted transaction:", tx);

      await confirmTransaction(provider.connection, tx);
    }
  }
}

main().catch((e) => {
  console.error(e);
  console.log(e.getLogs());
});
