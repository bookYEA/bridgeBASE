import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Bridge } from "../../target/types/bridge";
import { expect } from "chai";
import { confirmTransaction } from "../utils/confirmTransaction";
import { PublicKey } from "@solana/web3.js";
import { fundAccount } from "../utils/fundAccount";
import {
  dummyData,
  expectedBridgePubkey,
  expectedMessengerPubkey,
  minGasLimit,
  oracleSecretKey,
  otherBridgeAddress,
  otherMessengerAddress,
  solRemoteAddress,
  toAddress,
  VERSION,
} from "../utils/constants";
import { toNumberArray } from "../utils/toNumberArray";
import { deriveRoot } from "../utils/deriveRoot";
import { hashIxs, IxParam } from "../utils/hashIxs";
import { shouldFail } from "../utils/shouldFail";
import {
  createMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";

describe("receiver", () => {
  // Common test setup
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Bridge as Program<Bridge>;
  const payer = provider.wallet as anchor.Wallet;
  const oracle = anchor.web3.Keypair.fromSecretKey(oracleSecretKey);
  const TRANSFER_AMOUNT = 1 * anchor.web3.LAMPORTS_PER_SOL;

  // PDAs
  let messagePda: PublicKey;
  let rootPda: PublicKey;
  const [vaultPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("bridge_vault"), new anchor.BN(VERSION).toBuffer("le", 1)],
    program.programId
  );

  let proof: number[][];
  let nonce: number[];
  let sender: number[];

  // Arguments for MMR proof
  let leafIndexBN: anchor.BN;
  let totalLeafCountBN: anchor.BN;

  // Instructions
  let targetIxParam: IxParam;
  let messengerIxParam: IxParam;

  // Fixed transaction hashes for tests
  let transactionHash: number[];
  let transaction2: number[];
  let transaction3: number[];
  let transaction4: number[];

  /**
   * Helper function to serialize an instruction parameter
   */
  function serializeIxParam(ixParam: IxParam): Buffer {
    let serializedIxParam = Buffer.alloc(0);

    // Program ID
    serializedIxParam = Buffer.concat([
      serializedIxParam,
      ixParam.programId.toBuffer(),
    ]);

    // Accounts
    // Length of the accounts vector (u32 LE)
    const accountsLen = Buffer.alloc(4);
    accountsLen.writeUInt32LE(ixParam.accounts.length, 0);
    serializedIxParam = Buffer.concat([serializedIxParam, accountsLen]);

    for (const account of ixParam.accounts) {
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
    dataLen.writeUInt32LE(ixParam.data.length, 0);
    serializedIxParam = Buffer.concat([serializedIxParam, dataLen]);
    serializedIxParam = Buffer.concat([serializedIxParam, ixParam.data]);

    return serializedIxParam;
  }

  /**
   * Helper function to create a messenger payload
   */
  function createMessengerPayload(
    nonce: number[],
    sender: number[],
    ixParam: IxParam
  ): IxParam {
    const serializedIxParam = serializeIxParam(ixParam);

    // Construct the Vec<Ix> payload for MessengerPayload.message
    // If MessengerPayload.message is meant to be a Vec<Ix> containing one Ix,
    // it needs to be serialized as: [length_of_Vec<Ix> (u32le), serialized_bytes_of_Ix_0]
    const vecIxLengthBuffer = Buffer.alloc(4);
    vecIxLengthBuffer.writeUInt32LE(1, 0); // We have 1 instruction in this vector
    const message = Buffer.concat([vecIxLengthBuffer, serializedIxParam]);

    // Serialize MessengerPayload
    // Fields: nonce: [u8; 32], sender: [u8; 20], message: Vec<u8>
    const nonceBuffer = Buffer.from(nonce);
    const senderBuffer = Buffer.from(sender);

    // For MessengerPayload.message (which is itself a Vec<u8> containing the serialized Vec<Ix>)
    // Borsh expects u32 length prefix + data for this outer Vec<u8>
    const messengerPayloadMessageLenBuffer = Buffer.alloc(4);
    messengerPayloadMessageLenBuffer.writeUInt32LE(message.length, 0);

    const serializedMessengerPayload = Buffer.concat([
      nonceBuffer, // 32 bytes
      senderBuffer, // 20 bytes
      messengerPayloadMessageLenBuffer, // 4 bytes (length of message)
      message, // actual bytes of message (serialized Vec<Ix>)
    ]);

    const messengerIxParam = {
      programId: expectedMessengerPubkey,
      accounts: [],
      data: serializedMessengerPayload,
    };

    return messengerIxParam;
  }

  async function setupRootAndProof(
    blockNumber: anchor.BN,
    transactionHash: number[]
  ): Promise<{
    rootPda: PublicKey;
    proof: number[][];
    messagePda: PublicKey;
  }> {
    const [rootPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("output_root"), blockNumber.toBuffer("le", 8)],
      program.programId
    );

    const transactionsBatch = [
      transactionHash,
      transaction2,
      transaction3,
      transaction4,
    ];

    // Set MMR proof arguments based on the batch
    leafIndexBN = new anchor.BN(0); // transactionHash is the first leaf
    totalLeafCountBN = new anchor.BN(transactionsBatch.length);

    const { root, proof } = await deriveRoot(transactionsBatch);

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

    return { rootPda, proof, messagePda };
  }

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

    const transferIx = anchor.web3.SystemProgram.transfer({
      fromPubkey: payer.publicKey,
      toPubkey: oracle.publicKey,
      lamports: TRANSFER_AMOUNT,
    });
    targetIxParam = {
      programId: transferIx.programId,
      accounts: transferIx.keys,
      data: transferIx.data,
    };
    nonce = Array.from(Buffer.from(new anchor.BN(0).toArray("be", 32)));
    transactionHash = toNumberArray(hashIxs(nonce, toAddress, [targetIxParam]));

    const blockNumber = new anchor.BN(20);
    const result = await setupRootAndProof(blockNumber, transactionHash);
    rootPda = result.rootPda;
    proof = result.proof;
    messagePda = result.messagePda;
  });

  describe("Direct receiver transaction", () => {
    describe("Prove transaction", () => {
      it("Should fail if invalid transaction hash", async () => {
        await shouldFail(
          program.methods
            .proveTransaction(
              transaction2,
              nonce,
              toAddress,
              [targetIxParam],
              proof,
              leafIndexBN,
              totalLeafCountBN
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
              nonce,
              toAddress,
              [targetIxParam],
              badProof,
              leafIndexBN,
              totalLeafCountBN
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
            nonce,
            toAddress,
            [targetIxParam],
            proof,
            leafIndexBN,
            totalLeafCountBN
          )
          .accounts({ payer: payer.publicKey, root: rootPda })
          .rpc();

        await confirmTransaction(provider.connection, tx);

        const message = await program.account.message.fetch(messagePda);

        expect(message.ixs).to.eql([targetIxParam]);
        expect(message.isExecuted).to.be.false;
      });
    });

    describe("Finalize transaction", () => {
      before(async () => {
        const tx = await program.methods
          .finalizeTransaction(transactionHash)
          .accounts({})
          .remainingAccounts([
            ...targetIxParam.accounts,
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
              ...targetIxParam.accounts,
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

  describe("Relayed messenger transaction", () => {
    before(async () => {
      nonce = Array.from(Buffer.from(new anchor.BN(0).toArray("be", 32)));
      sender = otherBridgeAddress;

      messengerIxParam = createMessengerPayload(nonce, sender, targetIxParam);

      transactionHash = toNumberArray(
        hashIxs(nonce, otherMessengerAddress, [messengerIxParam])
      );

      const blockNumber = new anchor.BN(21);
      const result = await setupRootAndProof(blockNumber, transactionHash);
      rootPda = result.rootPda;
      proof = result.proof;
      messagePda = result.messagePda;
    });

    describe("Prove transction", () => {
      it("Should fail if invalid transaction hash", async () => {
        await shouldFail(
          program.methods
            .proveTransaction(
              transaction2,
              nonce,
              otherMessengerAddress,
              [messengerIxParam],
              proof,
              leafIndexBN,
              totalLeafCountBN
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
              nonce,
              otherMessengerAddress,
              [messengerIxParam],
              badProof,
              leafIndexBN,
              totalLeafCountBN
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
            nonce,
            otherMessengerAddress,
            [messengerIxParam],
            proof,
            leafIndexBN,
            totalLeafCountBN
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
            ...targetIxParam.accounts,
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
              ...targetIxParam.accounts,
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

  describe("Relayed bridge transaction", () => {
    let depositPda: PublicKey;
    let transferAccounts: {
      pubkey: PublicKey;
      isWritable: boolean;
      isSigner: boolean;
    }[];

    before(async () => {
      const mintAuthSC = anchor.web3.Keypair.generate();
      const mintKeypairSC = anchor.web3.Keypair.generate();

      const mintSC = await createMint(
        provider.connection,
        payer.payer,
        mintAuthSC.publicKey,
        mintAuthSC.publicKey,
        10,
        mintKeypairSC,
        undefined,
        TOKEN_PROGRAM_ID
      );

      const userATA = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        payer.payer,
        mintSC,
        payer.publicKey
      );

      const vaultATA = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        payer.payer,
        mintSC,
        vaultPda,
        true
      );

      await mintTo(
        provider.connection,
        payer.payer,
        mintSC,
        userATA.address,
        mintAuthSC,
        100 * anchor.web3.LAMPORTS_PER_SOL,
        [],
        undefined,
        TOKEN_PROGRAM_ID
      );

      const tx = await program.methods
        .bridgeTokensTo(
          solRemoteAddress,
          toAddress,
          new anchor.BN(TRANSFER_AMOUNT),
          minGasLimit,
          dummyData
        )
        .accounts({
          user: payer.publicKey,
          mint: mintSC,
        })
        .rpc();
      await confirmTransaction(provider.connection, tx);

      [depositPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("deposit"),
          mintSC.toBuffer(),
          Buffer.from(solRemoteAddress),
        ],
        program.programId
      );

      // Serialize BridgePayload
      // Fields: local_token: Pubkey, remote_token: [u8; 20], from: [u8; 20], to: Pubkey, amount: u64, extra_data: Vec<u8>
      const localTokenBuffer = mintSC.toBuffer();
      const remoteTokenBuffer = Buffer.from(solRemoteAddress);
      const fromBuffer = Buffer.from(toAddress);
      const toBuffer = userATA.address.toBuffer();
      const amountBuffer = new anchor.BN(TRANSFER_AMOUNT).toBuffer("le", 8);

      const extraDataBuffer = Buffer.from("random data", "utf-8");
      const extraDataLenBuffer = Buffer.alloc(4);
      extraDataLenBuffer.writeUint32LE(extraDataBuffer.length, 0);

      const serializedBridgePayload = Buffer.concat([
        localTokenBuffer,
        remoteTokenBuffer,
        fromBuffer,
        toBuffer,
        amountBuffer,
        extraDataLenBuffer,
        extraDataBuffer,
      ]);

      transferAccounts = [
        { pubkey: mintSC, isWritable: false, isSigner: false },
        { pubkey: vaultPda, isWritable: false, isSigner: false },
        { pubkey: vaultATA.address, isWritable: true, isSigner: false },
        { pubkey: userATA.address, isWritable: true, isSigner: false },
        { pubkey: TOKEN_PROGRAM_ID, isWritable: false, isSigner: false },
        { pubkey: depositPda, isWritable: true, isSigner: false },
      ];

      targetIxParam = {
        programId: expectedBridgePubkey,
        accounts: transferAccounts,
        data: serializedBridgePayload,
      };

      nonce = Array.from(Buffer.from(new anchor.BN(0).toArray("be", 32)));
      sender = otherBridgeAddress;

      messengerIxParam = createMessengerPayload(nonce, sender, targetIxParam);

      transactionHash = toNumberArray(
        hashIxs(nonce, otherMessengerAddress, [messengerIxParam])
      );

      const blockNumber = new anchor.BN(22);
      const result = await setupRootAndProof(blockNumber, transactionHash);
      rootPda = result.rootPda;
      proof = result.proof;
      messagePda = result.messagePda;
    });

    describe("Prove transction", () => {
      it("Should fail if invalid transaction hash", async () => {
        await shouldFail(
          program.methods
            .proveTransaction(
              transaction2,
              nonce,
              otherMessengerAddress,
              [messengerIxParam],
              proof,
              leafIndexBN,
              totalLeafCountBN
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
              nonce,
              otherMessengerAddress,
              [messengerIxParam],
              badProof,
              leafIndexBN,
              totalLeafCountBN
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
            nonce,
            otherMessengerAddress,
            [messengerIxParam],
            proof,
            leafIndexBN,
            totalLeafCountBN
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
          .remainingAccounts(transferAccounts)
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
            .remainingAccounts(transferAccounts)
            .rpc(),
          "Already executed"
        );
      });
    });
  });
});
