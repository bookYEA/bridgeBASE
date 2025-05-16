import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Bridge } from "../../target/types/bridge";
import { expect } from "chai";
import { confirmTransaction } from "../utils/confirmTransaction";
import { PublicKey } from "@solana/web3.js";
import { fundAccount } from "../utils/fundAccount";
import {
  expectedMessengerPubkey,
  oracleSecretKey,
  otherBridgeAddress,
  otherMessengerAddress,
  toAddress,
} from "../utils/constants";
import { toNumberArray } from "../utils/toNumberArray";
import { deriveRoot } from "../utils/deriveRoot";
import { hashIxs, IxParam } from "../utils/hashIxs";
import { shouldFail } from "../utils/shouldFail";

describe("receiver", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;
  const payer = provider.wallet;
  const oracle = anchor.web3.Keypair.fromSecretKey(oracleSecretKey);

  const blockNumber = new anchor.BN(20);

  const [rootPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("output_root"), blockNumber.toBuffer("le", 8)],
    program.programId
  );
  let messagePda: PublicKey;
  let proof: number[][];

  const TRANSFER_AMOUNT = 1 * anchor.web3.LAMPORTS_PER_SOL;

  const transferIx = anchor.web3.SystemProgram.transfer({
    fromPubkey: payer.publicKey,
    toPubkey: oracle.publicKey,
    lamports: TRANSFER_AMOUNT,
  });

  const transferIxParam: IxParam = {
    programId: transferIx.programId,
    accounts: transferIx.keys,
    data: transferIx.data,
  };
  const transactionHash = toNumberArray(hashIxs(toAddress, [transferIxParam]));
  let transaction2: number[];
  let transaction3: number[];
  let transaction4: number[];

  before(async () => {
    await fundAccount(provider, provider.wallet.publicKey, oracle.publicKey);

    transaction2 = toNumberArray(
      "0xb1f1d4e70a6c00ffb57d19be8bfe2dccc3695117af82b5a9183190b950fdd941"
    );
    transaction3 = toNumberArray(
      "0x513a04213b8de7fc313715c0bc14e6e2e9ab7bce369818597faf2612458d93ca"
    );
    transaction4 = toNumberArray(
      "0x8898b39e1f8771a1c07b2da4a191fabfc54de53b74c0fa1e82eea6de000bc424"
    );

    const transactionsBatch = [
      transactionHash,
      transaction2,
      transaction3,
      transaction4,
    ];

    const { root, proof: newProof } = await deriveRoot(transactionsBatch);
    proof = newProof;

    const tx = await program.methods
      .submitRoot(root as unknown as number[], blockNumber)
      .accounts({ payer: oracle.publicKey })
      .signers([oracle])
      .rpc();

    await confirmTransaction(provider.connection, tx);

    [messagePda] = PublicKey.findProgramAddressSync(
      [Buffer.from("message"), Buffer.from(transactionHash)],
      program.programId
    );
  });

  describe("Prove transaction", () => {
    it("Should fail if invalid transaction hash", async () => {
      await shouldFail(
        program.methods
          .proveTransaction(transaction2, toAddress, [transferIxParam], proof)
          .accounts({ payer: payer.publicKey, root: rootPda })
          .rpc(),
        "Invalid transaction hash"
      );
    });

    it("Should fail if invalid proof", async () => {
      const badProof = structuredClone(proof);
      badProof.pop();

      await shouldFail(
        program.methods
          .proveTransaction(
            transactionHash,
            toAddress,
            [transferIxParam],
            badProof
          )
          .accounts({ payer: payer.publicKey, root: rootPda })
          .rpc(),
        "Invalid proof"
      );
    });

    it("Posts output root", async () => {
      const tx = await program.methods
        .proveTransaction(transactionHash, toAddress, [transferIxParam], proof)
        .accounts({ payer: payer.publicKey, root: rootPda })
        .rpc();

      await confirmTransaction(provider.connection, tx);

      const message = await program.account.message.fetch(messagePda);

      expect(message.ixs).to.eql([transferIxParam]);
      expect(message.isExecuted).to.be.false;
    });
  });

  describe("Finalize transaction", () => {
    before(async () => {
      const tx = await program.methods
        .finalizeTransaction(transactionHash)
        .accounts({})
        .remainingAccounts([
          ...transferIxParam.accounts,
          {
            pubkey: anchor.web3.SystemProgram.programId,
            isSigner: false,
            isWritable: false,
          },
        ])
        .rpc();

      await confirmTransaction(provider.connection, tx);
    });

    it("Should execute transaction", async () => {
      const message = await program.account.message.fetch(messagePda);
      expect(message.isExecuted).to.be.true;
    });

    it("Should fail if already executed", async () => {
      await shouldFail(
        program.methods
          .finalizeTransaction(transactionHash)
          .accounts({})
          .remainingAccounts([
            ...transferIxParam.accounts,
            {
              pubkey: anchor.web3.SystemProgram.programId,
              isSigner: false,
              isWritable: false,
            },
          ])
          .rpc(),
        "Already executed"
      );
    });
  });

  describe("Relayed messenger transaction", () => {
    const blockNumber = new anchor.BN(21);
    const [rootPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("output_root"), blockNumber.toBuffer("le", 8)],
      program.programId
    );
    let nonce: number[];
    let sender: number[];
    let message: Buffer;
    let serializedMessengerPayload: Buffer;

    let messengerIxParam: IxParam;
    let transactionHash: number[];

    before(async () => {
      // Encode transferIxParam into Vec<u8>
      let serializedIxParam = Buffer.alloc(0);

      // Program ID
      serializedIxParam = Buffer.concat([
        serializedIxParam,
        transferIxParam.programId.toBuffer(),
      ]);

      // Accounts
      // Length of the accounts vector (u32 LE)
      const accountsLen = Buffer.alloc(4);
      accountsLen.writeUInt32LE(transferIxParam.accounts.length, 0);
      serializedIxParam = Buffer.concat([serializedIxParam, accountsLen]);

      for (const account of transferIxParam.accounts) {
        serializedIxParam = Buffer.concat([
          serializedIxParam,
          account.pubkey.toBuffer(),
        ]);
        serializedIxParam = Buffer.concat([
          serializedIxParam,
          Buffer.from([account.isWritable ? 1 : 0]),
        ]);
        serializedIxParam = Buffer.concat([
          serializedIxParam,
          Buffer.from([account.isSigner ? 1 : 0]),
        ]);
      }

      // Data
      // Length of the data vector (u32 LE)
      const dataLen = Buffer.alloc(4);
      dataLen.writeUInt32LE(transferIxParam.data.length, 0);
      serializedIxParam = Buffer.concat([serializedIxParam, dataLen]);
      serializedIxParam = Buffer.concat([
        serializedIxParam,
        transferIxParam.data,
      ]);

      nonce = Array.from(Buffer.from(new anchor.BN(0).toArray("be", 32)));
      sender = otherBridgeAddress;

      // NEW: Construct the Vec<Ix> payload for MessengerPayload.message
      // If MessengerPayload.message is meant to be a Vec<Ix> containing one Ix (transferIxParam),
      // it needs to be serialized as: [length_of_Vec<Ix> (u32le), serialized_bytes_of_Ix_0]
      const vecIxLengthBuffer = Buffer.alloc(4);
      vecIxLengthBuffer.writeUInt32LE(1, 0); // We have 1 instruction in this vector
      message = Buffer.concat([vecIxLengthBuffer, serializedIxParam]);

      // Serialize MessengerPayload
      // Fields: nonce: [u8; 32], sender: [u8; 20], message: Vec<u8>
      const nonceBuffer = Buffer.from(nonce);
      const senderBuffer = Buffer.from(sender); // This is a 20-byte Buffer

      // For MessengerPayload.message (which is itself a Vec<u8> containing the serialized Vec<Ix>)
      // Borsh expects u32 length prefix + data for this outer Vec<u8>
      const messengerPayloadMessageLenBuffer = Buffer.alloc(4);
      messengerPayloadMessageLenBuffer.writeUInt32LE(message.length, 0);

      serializedMessengerPayload = Buffer.concat([
        nonceBuffer, // 32 bytes
        senderBuffer, // 20 bytes
        messengerPayloadMessageLenBuffer, // 4 bytes (length of message)
        message, // actual bytes of message (serialized Vec<Ix>)
      ]);

      messengerIxParam = {
        programId: expectedMessengerPubkey,
        accounts: [],
        data: serializedMessengerPayload,
      };

      transactionHash = toNumberArray(
        hashIxs(otherMessengerAddress, [messengerIxParam])
      );

      const transactionsBatch = [
        transactionHash,
        transaction2,
        transaction3,
        transaction4,
      ];

      const { root, proof: newProof } = await deriveRoot(transactionsBatch);
      proof = newProof;

      const tx = await program.methods
        .submitRoot(root as unknown as number[], blockNumber)
        .accounts({ payer: oracle.publicKey })
        .signers([oracle])
        .rpc();

      await confirmTransaction(provider.connection, tx);

      [messagePda] = PublicKey.findProgramAddressSync(
        [Buffer.from("message"), Buffer.from(transactionHash)],
        program.programId
      );
    });

    describe("Prove transction", () => {
      it("Should fail if invalid transaction hash", async () => {
        await shouldFail(
          program.methods
            .proveTransaction(
              transaction2,
              otherMessengerAddress,
              [messengerIxParam],
              proof
            )
            .accounts({ payer: payer.publicKey, root: rootPda })
            .rpc(),
          "Invalid transaction hash"
        );
      });

      it("Should fail if invalid proof", async () => {
        const badProof = structuredClone(proof);
        badProof.pop();

        await shouldFail(
          program.methods
            .proveTransaction(
              transactionHash,
              otherMessengerAddress,
              [messengerIxParam],
              badProof
            )
            .accounts({ payer: payer.publicKey, root: rootPda })
            .rpc(),
          "Invalid proof"
        );
      });

      it("Posts output root", async () => {
        const tx = await program.methods
          .proveTransaction(
            transactionHash,
            otherMessengerAddress,
            [messengerIxParam],
            proof
          )
          .accounts({ payer: payer.publicKey, root: rootPda })
          .rpc();

        await confirmTransaction(provider.connection, tx);

        const message = await program.account.message.fetch(messagePda);

        expect(message.ixs).to.eql([messengerIxParam]);
        expect(message.isExecuted).to.be.false;
      });
    });

    describe("Finalize transaction", () => {
      before(async () => {
        const tx = await program.methods
          .finalizeTransaction(transactionHash)
          .accounts({})
          .remainingAccounts([
            ...transferIxParam.accounts,
            {
              pubkey: anchor.web3.SystemProgram.programId,
              isSigner: false,
              isWritable: false,
            },
          ])
          .rpc();

        await confirmTransaction(provider.connection, tx);
      });

      it("Should execute transaction", async () => {
        const message = await program.account.message.fetch(messagePda);
        expect(message.isExecuted).to.be.true;
      });

      it("Should mark message successful", async () => {
        const message = await program.account.message.fetch(messagePda);
        expect(message.successfulMessage).to.be.true;
      });

      it("Should fail if already executed", async () => {
        await shouldFail(
          program.methods
            .finalizeTransaction(transactionHash)
            .accounts({})
            .remainingAccounts([
              ...transferIxParam.accounts,
              {
                pubkey: anchor.web3.SystemProgram.programId,
                isSigner: false,
                isWritable: false,
              },
            ])
            .rpc(),
          "Already executed"
        );
      });
    });
  });
});
