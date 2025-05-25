import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Bridge } from "../../target/types/bridge";
import { expect } from "chai";
import { confirmTransaction } from "../utils/confirmTransaction";
import { PublicKey } from "@solana/web3.js";
import { fundAccount } from "../utils/fundAccount";
import {
  decimals,
  dummyData,
  expectedBridgePubkey,
  minGasLimit,
  oracleSecretKey,
  otherBridgeAddress,
  otherMessengerAddress,
  solRemoteAddress,
  toAddress,
  VERSION,
} from "../utils/constants";
import { toNumberArray } from "../utils/toNumberArray";
import { hashIxs, IxParam } from "../utils/hashIxs";
import { shouldFail } from "../utils/shouldFail";
import {
  createMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { SYSTEM_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/native/system";
import { createMessengerPayload } from "../utils/createMessengerPayload";
import { setupRootAndProof } from "../utils/setupRootAndProof";

describe("receiver", () => {
  // Common test setup
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Bridge as Program<Bridge>;
  const payer = provider.wallet as anchor.Wallet;
  const oracle = anchor.web3.Keypair.fromSecretKey(oracleSecretKey);
  const TRANSFER_AMOUNT = 100 * anchor.web3.LAMPORTS_PER_SOL;

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

  before(async () => {
    await fundAccount(provider, provider.wallet.publicKey, oracle.publicKey);

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
    const result = await setupRootAndProof(
      program,
      blockNumber,
      transactionHash
    );
    rootPda = result.rootPda;
    proof = result.proof;
    messagePda = result.messagePda;
    leafIndexBN = result.leafIndexBN;
    totalLeafCountBN = result.totalLeafCountBN;
    transaction2 = result.transaction2;
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
      const result = await setupRootAndProof(
        program,
        blockNumber,
        transactionHash
      );
      rootPda = result.rootPda;
      proof = result.proof;
      messagePda = result.messagePda;
      leafIndexBN = result.leafIndexBN;
      totalLeafCountBN = result.totalLeafCountBN;
      transaction2 = result.transaction2;
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

  describe("Relayed SPL bridge transaction", () => {
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
        accounts: [],
        data: serializedBridgePayload,
      };

      nonce = Array.from(Buffer.from(new anchor.BN(0).toArray("be", 32)));
      sender = otherBridgeAddress;

      messengerIxParam = createMessengerPayload(nonce, sender, targetIxParam);

      transactionHash = toNumberArray(
        hashIxs(nonce, otherMessengerAddress, [messengerIxParam])
      );

      const blockNumber = new anchor.BN(22);
      const result = await setupRootAndProof(
        program,
        blockNumber,
        transactionHash
      );
      rootPda = result.rootPda;
      proof = result.proof;
      messagePda = result.messagePda;
      leafIndexBN = result.leafIndexBN;
      totalLeafCountBN = result.totalLeafCountBN;
      transaction2 = result.transaction2;
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

  describe("Relayed SOL bridge transaction", () => {
    let depositPda: PublicKey;
    let transferAccounts: {
      pubkey: PublicKey;
      isWritable: boolean;
      isSigner: boolean;
    }[];

    const solLocalAddress = new PublicKey(
      Buffer.from(
        "0501550155015501550155015501550155015501550155015501550155015501",
        "hex"
      )
    );

    before(async () => {
      const tx = await program.methods
        .bridgeSolTo(
          solRemoteAddress,
          toAddress,
          new anchor.BN(TRANSFER_AMOUNT),
          minGasLimit,
          dummyData
        )
        .accounts({ user: payer.publicKey })
        .rpc();
      await confirmTransaction(provider.connection, tx);

      [depositPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("deposit"),
          solLocalAddress.toBuffer(),
          Buffer.from(solRemoteAddress),
        ],
        program.programId
      );

      // Serialize BridgePayload
      // Fields: local_token: Pubkey, remote_token: [u8; 20], from: [u8; 20], to: Pubkey, amount: u64, extra_data: Vec<u8>
      const localTokenBuffer = solLocalAddress.toBuffer();
      const remoteTokenBuffer = Buffer.from(solRemoteAddress);
      const fromBuffer = Buffer.from(toAddress);
      const toBuffer = payer.publicKey.toBuffer();
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
        { pubkey: payer.publicKey, isWritable: true, isSigner: false },
        { pubkey: SYSTEM_PROGRAM_ID, isWritable: false, isSigner: false },
        { pubkey: depositPda, isWritable: true, isSigner: false },
      ];

      targetIxParam = {
        programId: expectedBridgePubkey,
        accounts: [],
        data: serializedBridgePayload,
      };

      nonce = Array.from(Buffer.from(new anchor.BN(0).toArray("be", 32)));
      sender = otherBridgeAddress;

      messengerIxParam = createMessengerPayload(nonce, sender, targetIxParam);

      transactionHash = toNumberArray(
        hashIxs(nonce, otherMessengerAddress, [messengerIxParam])
      );

      const blockNumber = new anchor.BN(23);
      const result = await setupRootAndProof(
        program,
        blockNumber,
        transactionHash
      );
      rootPda = result.rootPda;
      proof = result.proof;
      messagePda = result.messagePda;
      leafIndexBN = result.leafIndexBN;
      totalLeafCountBN = result.totalLeafCountBN;
      transaction2 = result.transaction2;
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

  describe("Returned SPL bridge transaction", () => {
    let depositPda: PublicKey;
    let transferAccounts: {
      pubkey: PublicKey;
      isWritable: boolean;
      isSigner: boolean;
    }[];

    const remoteTokenAddress = Array.from(
      Buffer.from("7aBc6d57A03f3b3eeA91fc2151638A549050eB42", "hex")
    );

    const [mintPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("mint"),
        Buffer.from(remoteTokenAddress),
        new anchor.BN(decimals).toBuffer("le", 1),
      ],
      program.programId
    );

    before(async () => {
      const tx = await program.methods
        .createMint(remoteTokenAddress, decimals)
        .accounts({ tokenProgram: TOKEN_PROGRAM_ID })
        .rpc();

      await confirmTransaction(provider.connection, tx);

      [depositPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("deposit"),
          mintPda.toBuffer(),
          Buffer.from(remoteTokenAddress),
        ],
        program.programId
      );

      const userATA = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        payer.payer,
        mintPda,
        payer.publicKey
      );

      // Serialize BridgePayload
      // Fields: local_token: Pubkey, remote_token: [u8; 20], from: [u8; 20], to: Pubkey, amount: u64, extra_data: Vec<u8>
      const localTokenBuffer = mintPda.toBuffer();
      const remoteTokenBuffer = Buffer.from(remoteTokenAddress);
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
        { pubkey: userATA.address, isWritable: true, isSigner: false },
        { pubkey: mintPda, isWritable: true, isSigner: false },
        { pubkey: TOKEN_PROGRAM_ID, isWritable: false, isSigner: false },
      ];

      targetIxParam = {
        programId: expectedBridgePubkey,
        accounts: [],
        data: serializedBridgePayload,
      };

      nonce = Array.from(Buffer.from(new anchor.BN(0).toArray("be", 32)));
      sender = otherBridgeAddress;

      messengerIxParam = createMessengerPayload(nonce, sender, targetIxParam);

      transactionHash = toNumberArray(
        hashIxs(nonce, otherMessengerAddress, [messengerIxParam])
      );

      const blockNumber = new anchor.BN(24);
      const result = await setupRootAndProof(
        program,
        blockNumber,
        transactionHash
      );
      rootPda = result.rootPda;
      proof = result.proof;
      messagePda = result.messagePda;
      leafIndexBN = result.leafIndexBN;
      totalLeafCountBN = result.totalLeafCountBN;
      transaction2 = result.transaction2;
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
