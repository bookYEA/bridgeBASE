import { expect } from "chai";
import { PublicKey } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";

import { Bridge } from "../../target/types/bridge";

import { confirmTransaction } from "../utils/confirmTransaction";
import { fundAccount } from "../utils/fundAccount";
import {
  DUMMY_DATA,
  MIN_GAS_LIMIT,
  ORACLE_SECRET_KEY,
  programConstant,
  REMOTE_TOKEN_ADDRESS,
} from "../utils/constants";
import { hashIxs } from "../utils/hashIxs";
import { shouldFail } from "../utils/shouldFail";
import { createMessengerIxParam } from "../utils/payloads";
import {
  setupRootAndProof,
  SetupRootAndProofResult,
} from "../utils/setupRootAndProof";
import { createBridgeIxParam } from "../utils/payloads";
import {
  createAssociatedTokenAccountIdempotent,
  createMint,
  getAssociatedTokenAddressSync,
  mintTo,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { printLogs } from "../utils/printLogs";

describe("receiver", () => {
  // Common test setup
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Bridge as Program<Bridge>;
  const user = provider.wallet as anchor.Wallet;

  // Test constants
  const ORACLE = anchor.web3.Keypair.fromSecretKey(ORACLE_SECRET_KEY);
  const TRANSFER_AMOUNT = 100 * anchor.web3.LAMPORTS_PER_SOL;
  const RECEIVER = anchor.web3.Keypair.generate();
  const DECIMALS = 9;

  // Program constants
  const DEFAULT_MESSENGER_CALLER = programConstant("defaultMessengerCaller");
  const REMOTE_BRIDGE = programConstant("remoteBridge");
  const REMOTE_MESSENGER = programConstant("remoteMessenger");
  const DEPOSIT_SEED = programConstant("depositSeed");
  const NATIVE_SOL_PUBKEY = programConstant("nativeSolPubkey");
  const TOKEN_VAULT_SEED = programConstant("tokenVaultSeed");
  const VERSION = programConstant("version");
  const MINT_SEED = programConstant("mintSeed");

  before(async () => {
    await fundAccount({
      provider,
      from: provider.wallet.publicKey,
      to: ORACLE.publicKey,
    });
  });

  describe("Direct receiver transaction", () => {
    const transferIx = anchor.web3.SystemProgram.transfer({
      fromPubkey: user.publicKey,
      toPubkey: RECEIVER.publicKey,
      lamports: TRANSFER_AMOUNT,
    });

    const transferIxParam = {
      programId: transferIx.programId,
      accounts: transferIx.keys,
      data: transferIx.data,
    };

    const REMOTE_SENDER = Array.from({ length: 20 }, (_, i) => i);
    const nonce = Array.from(Buffer.from(new anchor.BN(0).toArray("be", 32)));
    const transactionHash = hashIxs({
      nonce,
      remoteSender: REMOTE_SENDER,
      ixs: [transferIxParam],
    });

    let setupResult: SetupRootAndProofResult;

    before(async () => {
      const blockNumber = new anchor.BN(20);
      setupResult = await setupRootAndProof({
        program,
        blockNumber,
        transactionHash,
      });
    });

    describe("Prove transaction", () => {
      it("Should fail if invalid transaction hash", async () => {
        await shouldFail({
          fn: program.methods
            .proveTransaction(
              setupResult.transaction2,
              nonce,
              REMOTE_SENDER,
              [transferIxParam],
              setupResult.proof,
              setupResult.leafIndexBN,
              setupResult.totalLeafCountBN
            )
            .accounts({ payer: user.publicKey, root: setupResult.rootPda })
            .rpc(),
          expectedError: "Invalid transaction hash",
        });
      });

      it("Should fail if invalid proof", async () => {
        const badProof = structuredClone(setupResult.proof);
        badProof.pop();

        await shouldFail({
          fn: program.methods
            .proveTransaction(
              transactionHash,
              nonce,
              REMOTE_SENDER,
              [transferIxParam],
              badProof,
              setupResult.leafIndexBN,
              setupResult.totalLeafCountBN
            )
            .accounts({ payer: user.publicKey, root: setupResult.rootPda })
            .rpc(),
          expectedError: "Invalid proof",
        });
      });

      it("Should create and initialize a message", async () => {
        const tx = await program.methods
          .proveTransaction(
            transactionHash,
            nonce,
            REMOTE_SENDER,
            [transferIxParam],
            setupResult.proof,
            setupResult.leafIndexBN,
            setupResult.totalLeafCountBN
          )
          .accounts({ payer: user.publicKey, root: setupResult.rootPda })
          .rpc();

        await confirmTransaction({ connection: provider.connection, tx });

        const message = await program.account.message.fetch(
          setupResult.messagePda
        );

        expect(message.isExecuted).to.be.false;
        expect(message.failedMessage).to.be.false;
        expect(message.successfulMessage).to.be.false;
        expect(message.messagePasserCaller).to.deep.equal(REMOTE_SENDER);
        expect(message.messengerCaller).to.deep.equal(DEFAULT_MESSENGER_CALLER);
        expect(message.ixs).to.eql([transferIxParam]);
      });
    });

    describe("Finalize transaction", () => {
      let userBalanceBefore: number;
      let receiverBalanceBefore: number;

      before(async () => {
        userBalanceBefore = await provider.connection.getBalance(
          user.publicKey
        );

        receiverBalanceBefore = await provider.connection.getBalance(
          RECEIVER.publicKey
        );

        const tx = await program.methods
          .finalizeTransaction()
          .accounts({
            message: setupResult.messagePda,
          })
          .remainingAccounts([
            ...transferIxParam.accounts,
            {
              pubkey: anchor.web3.SystemProgram.programId,
              isSigner: false,
              isWritable: false,
            },
          ])
          .rpc();

        await confirmTransaction({ connection: provider.connection, tx });
      });

      it("Should execute transaction", async () => {
        const message = await program.account.message.fetch(
          setupResult.messagePda
        );
        expect(message.isExecuted).to.be.true;
      });

      it("Should transfer SOL from user to receiver", async () => {
        const userBalanceAfter = await provider.connection.getBalance(
          user.publicKey
        );

        const receiverBalanceAfter = await provider.connection.getBalance(
          RECEIVER.publicKey
        );

        expect(userBalanceAfter).to.be.lessThan(
          userBalanceBefore - TRANSFER_AMOUNT
        );
        expect(receiverBalanceAfter).to.be.equal(
          receiverBalanceBefore + TRANSFER_AMOUNT
        );
      });

      it("Should fail if already executed", async () => {
        await shouldFail({
          fn: program.methods
            .finalizeTransaction()
            .accounts({
              message: setupResult.messagePda,
            })
            .remainingAccounts([
              ...transferIxParam.accounts,
              {
                pubkey: anchor.web3.SystemProgram.programId,
                isSigner: false,
                isWritable: false,
              },
            ])
            .rpc(),
          expectedError: "Already executed",
        });
      });
    });
  });

  describe("Relayed messenger transaction", () => {
    const transferIx = anchor.web3.SystemProgram.transfer({
      fromPubkey: user.publicKey,
      toPubkey: RECEIVER.publicKey,
      lamports: TRANSFER_AMOUNT,
    });

    const transferIxParam = {
      programId: transferIx.programId,
      accounts: transferIx.keys,
      data: transferIx.data,
    };

    const nonce = Array.from(Buffer.from(new anchor.BN(0).toArray("be", 32)));
    const REMOTE_SENDER = Array.from({ length: 20 }, (_, i) => i);

    const messengerIxParam = createMessengerIxParam({
      nonce,
      sender: REMOTE_SENDER,
      ixParam: transferIxParam,
    });

    const transactionHash = hashIxs({
      nonce,
      remoteSender: REMOTE_MESSENGER,
      ixs: [messengerIxParam],
    });

    let setupResult: SetupRootAndProofResult;

    before(async () => {
      const blockNumber = new anchor.BN(21);
      setupResult = await setupRootAndProof({
        program,
        blockNumber,
        transactionHash,
      });
    });

    describe("Prove transction", () => {
      it("Should fail if invalid transaction hash", async () => {
        await shouldFail({
          fn: program.methods
            .proveTransaction(
              setupResult.transaction2,
              nonce,
              REMOTE_MESSENGER,
              [messengerIxParam],
              setupResult.proof,
              setupResult.leafIndexBN,
              setupResult.totalLeafCountBN
            )
            .accounts({ payer: user.publicKey, root: setupResult.rootPda })
            .rpc(),
          expectedError: "Invalid transaction hash",
        });
      });

      it("Should fail if invalid proof", async () => {
        const badProof = structuredClone(setupResult.proof);
        badProof.pop();

        await shouldFail({
          fn: program.methods
            .proveTransaction(
              transactionHash,
              nonce,
              REMOTE_MESSENGER,
              [messengerIxParam],
              badProof,
              setupResult.leafIndexBN,
              setupResult.totalLeafCountBN
            )
            .accounts({ payer: user.publicKey, root: setupResult.rootPda })
            .rpc(),
          expectedError: "Invalid proof",
        });
      });

      it("Should create and initialize a message", async () => {
        const tx = await program.methods
          .proveTransaction(
            transactionHash,
            nonce,
            REMOTE_MESSENGER,
            [messengerIxParam],
            setupResult.proof,
            setupResult.leafIndexBN,
            setupResult.totalLeafCountBN
          )
          .accounts({ payer: user.publicKey, root: setupResult.rootPda })
          .rpc();

        await confirmTransaction({ connection: provider.connection, tx });

        const message = await program.account.message.fetch(
          setupResult.messagePda
        );

        expect(message.isExecuted).to.be.false;
        expect(message.failedMessage).to.be.false;
        expect(message.successfulMessage).to.be.false;
        expect(message.messagePasserCaller).to.deep.equal(REMOTE_MESSENGER);
        expect(message.messengerCaller).to.deep.equal(DEFAULT_MESSENGER_CALLER);
        expect(message.ixs).to.eql([messengerIxParam]);
      });
    });

    describe("Finalize transaction", () => {
      let userBalanceBefore: number;
      let receiverBalanceBefore: number;

      before(async () => {
        userBalanceBefore = await provider.connection.getBalance(
          user.publicKey
        );

        receiverBalanceBefore = await provider.connection.getBalance(
          RECEIVER.publicKey
        );

        const tx = await program.methods
          .finalizeTransaction()
          .accounts({
            message: setupResult.messagePda,
          })
          .remainingAccounts([
            ...transferIxParam.accounts,
            {
              pubkey: anchor.web3.SystemProgram.programId,
              isSigner: false,
              isWritable: false,
            },
          ])
          .rpc();

        await confirmTransaction({ connection: provider.connection, tx });
      });

      it("Should execute transaction", async () => {
        const message = await program.account.message.fetch(
          setupResult.messagePda
        );
        expect(message.isExecuted).to.be.true;
      });

      it("Should mark message successful", async () => {
        const message = await program.account.message.fetch(
          setupResult.messagePda
        );
        expect(message.successfulMessage).to.be.true;
      });

      it("Should transfer SOL from user to receiver", async () => {
        const userBalanceAfter = await provider.connection.getBalance(
          user.publicKey
        );
        const receiverBalanceAfter = await provider.connection.getBalance(
          RECEIVER.publicKey
        );

        expect(userBalanceAfter).to.be.lessThan(
          userBalanceBefore - TRANSFER_AMOUNT
        );

        expect(receiverBalanceAfter).to.be.equal(
          receiverBalanceBefore + TRANSFER_AMOUNT
        );
      });

      it("Should fail if already executed", async () => {
        await shouldFail({
          fn: program.methods
            .finalizeTransaction()
            .accounts({
              message: setupResult.messagePda,
            })
            .remainingAccounts([
              ...transferIxParam.accounts,
              {
                pubkey: anchor.web3.SystemProgram.programId,
                isSigner: false,
                isWritable: false,
              },
            ])
            .rpc(),
          expectedError: "Already executed",
        });
      });
    });
  });

  describe("Relayed SOL bridge transaction", () => {
    const FROM = Array.from({ length: 20 }, (_, i) => i);

    const [depositPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from(DEPOSIT_SEED),
        NATIVE_SOL_PUBKEY.toBuffer(),
        Buffer.from(REMOTE_TOKEN_ADDRESS),
      ],
      program.programId
    );

    const finalizeSolBridgeAccounts = [
      { pubkey: user.publicKey, isWritable: true, isSigner: false },
      { pubkey: depositPda, isWritable: true, isSigner: false },
    ];

    const bridgeIxParam = createBridgeIxParam({
      localToken: NATIVE_SOL_PUBKEY,
      remoteToken: REMOTE_TOKEN_ADDRESS,
      from: FROM,
      to: user.publicKey,
      amount: new anchor.BN(TRANSFER_AMOUNT),
      extraData: Buffer.from("random data", "utf-8"),
    });

    const nonce = Array.from(Buffer.from(new anchor.BN(0).toArray("be", 32)));

    const messengerIxParam = createMessengerIxParam({
      nonce,
      sender: REMOTE_BRIDGE,
      ixParam: bridgeIxParam,
    });

    const transactionHash = hashIxs({
      nonce,
      remoteSender: REMOTE_MESSENGER,
      ixs: [messengerIxParam],
    });

    let setupResult: SetupRootAndProofResult;

    before(async () => {
      const tx = await program.methods
        .bridgeSolTo(
          REMOTE_TOKEN_ADDRESS,
          FROM,
          new anchor.BN(TRANSFER_AMOUNT),
          MIN_GAS_LIMIT,
          DUMMY_DATA
        )
        .accounts({ user: user.publicKey })
        .rpc();

      await confirmTransaction({ connection: provider.connection, tx });

      const blockNumber = new anchor.BN(23);
      setupResult = await setupRootAndProof({
        program,
        blockNumber,
        transactionHash,
      });
    });

    describe("Prove transction", () => {
      it("Should fail if invalid transaction hash", async () => {
        await shouldFail({
          fn: program.methods
            .proveTransaction(
              setupResult.transaction2,
              nonce,
              REMOTE_MESSENGER,
              [messengerIxParam],
              setupResult.proof,
              setupResult.leafIndexBN,
              setupResult.totalLeafCountBN
            )
            .accounts({ payer: user.publicKey, root: setupResult.rootPda })
            .rpc(),
          expectedError: "Invalid transaction hash",
        });
      });

      it("Should fail if invalid proof", async () => {
        const badProof = structuredClone(setupResult.proof);
        badProof.pop();

        await shouldFail({
          fn: program.methods
            .proveTransaction(
              transactionHash,
              nonce,
              REMOTE_MESSENGER,
              [messengerIxParam],
              badProof,
              setupResult.leafIndexBN,
              setupResult.totalLeafCountBN
            )
            .accounts({ payer: user.publicKey, root: setupResult.rootPda })
            .rpc(),
          expectedError: "Invalid proof",
        });
      });

      it("Posts output root", async () => {
        const tx = await program.methods
          .proveTransaction(
            transactionHash,
            nonce,
            REMOTE_MESSENGER,
            [messengerIxParam],
            setupResult.proof,
            setupResult.leafIndexBN,
            setupResult.totalLeafCountBN
          )
          .accounts({ payer: user.publicKey, root: setupResult.rootPda })
          .rpc();

        await confirmTransaction({ connection: provider.connection, tx });

        const message = await program.account.message.fetch(
          setupResult.messagePda
        );

        expect(message.ixs).to.eql([messengerIxParam]);
        expect(message.isExecuted).to.be.false;
      });
    });

    describe("Finalize transaction", () => {
      let userBalanceBefore: number;

      before(async () => {
        userBalanceBefore = await provider.connection.getBalance(
          user.publicKey
        );

        const tx = await program.methods
          .finalizeTransaction()
          .accounts({
            message: setupResult.messagePda,
          })
          .remainingAccounts(finalizeSolBridgeAccounts)
          .rpc();

        await confirmTransaction({ connection: provider.connection, tx });
      });

      it("Should execute transaction", async () => {
        const message = await program.account.message.fetch(
          setupResult.messagePda
        );
        expect(message.isExecuted).to.be.true;
      });

      it("Should mark message successful", async () => {
        const message = await program.account.message.fetch(
          setupResult.messagePda
        );
        expect(message.successfulMessage).to.be.true;
      });

      it("Should transfer SOL back to user", async () => {
        const userBalanceAfter = await provider.connection.getBalance(
          user.publicKey
        );

        expect(userBalanceAfter).to.be.greaterThan(userBalanceBefore);
      });

      it("Should fail if already executed", async () => {
        await shouldFail({
          fn: program.methods
            .finalizeTransaction()
            .accounts({
              message: setupResult.messagePda,
            })
            .remainingAccounts(finalizeSolBridgeAccounts)
            .rpc(),
          expectedError: "Already executed",
        });
      });
    });
  });

  describe("Relayed SPL bridge transaction", () => {
    const MINT_AUTH = anchor.web3.Keypair.generate();
    const MINT_KEYPAIR = anchor.web3.Keypair.generate();
    const MINT = MINT_KEYPAIR.publicKey;
    const FROM = Array.from({ length: 20 }, (_, i) => i);

    const [depositPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from(DEPOSIT_SEED),
        MINT.toBuffer(),
        Buffer.from(REMOTE_TOKEN_ADDRESS),
      ],
      program.programId
    );

    const userTokenAccount = getAssociatedTokenAddressSync(
      MINT,
      user.publicKey,
      false
    );

    const [tokenVault] = PublicKey.findProgramAddressSync(
      [
        Buffer.from(TOKEN_VAULT_SEED),
        MINT.toBuffer(),
        new anchor.BN(VERSION).toBuffer("le", 1),
      ],
      program.programId
    );

    const finalizeSplBridgeAccounts = [
      { pubkey: userTokenAccount, isWritable: true, isSigner: false },
      { pubkey: MINT, isWritable: false, isSigner: false },
      { pubkey: depositPda, isWritable: true, isSigner: false },
      { pubkey: tokenVault, isWritable: true, isSigner: false },
      { pubkey: TOKEN_PROGRAM_ID, isWritable: false, isSigner: false },
    ];

    const bridgeIxParam = createBridgeIxParam({
      localToken: MINT,
      remoteToken: REMOTE_TOKEN_ADDRESS,
      from: FROM,
      to: userTokenAccount,
      amount: new anchor.BN(TRANSFER_AMOUNT),
      extraData: Buffer.from("random data", "utf-8"),
    });

    const nonce = Array.from(Buffer.from(new anchor.BN(0).toArray("be", 32)));

    const messengerIxParam = createMessengerIxParam({
      nonce,
      sender: REMOTE_BRIDGE,
      ixParam: bridgeIxParam,
    });

    const transactionHash = hashIxs({
      nonce,
      remoteSender: REMOTE_MESSENGER,
      ixs: [messengerIxParam],
    });

    let setupResult: SetupRootAndProofResult;

    before(async () => {
      const mint = await createMint(
        provider.connection,
        user.payer,
        MINT_AUTH.publicKey,
        MINT_AUTH.publicKey,
        DECIMALS,
        MINT_KEYPAIR
      );

      await createAssociatedTokenAccountIdempotent(
        provider.connection,
        user.payer,
        MINT,
        user.publicKey
      );

      await mintTo(
        provider.connection,
        user.payer,
        mint,
        userTokenAccount,
        MINT_AUTH,
        100 * anchor.web3.LAMPORTS_PER_SOL
      );

      const tx = await program.methods
        .bridgeTokensTo(
          REMOTE_TOKEN_ADDRESS,
          FROM,
          new anchor.BN(TRANSFER_AMOUNT),
          MIN_GAS_LIMIT,
          DUMMY_DATA
        )
        .accounts({
          user: user.publicKey,
          mint: mint,
          fromTokenAccount: userTokenAccount,
        })
        .rpc();

      await confirmTransaction({ connection: provider.connection, tx });

      const blockNumber = new anchor.BN(22);
      setupResult = await setupRootAndProof({
        program,
        blockNumber,
        transactionHash,
      });
    });

    describe("Prove transction", () => {
      it("Should fail if invalid transaction hash", async () => {
        await shouldFail({
          fn: program.methods
            .proveTransaction(
              setupResult.transaction2,
              nonce,
              REMOTE_MESSENGER,
              [messengerIxParam],
              setupResult.proof,
              setupResult.leafIndexBN,
              setupResult.totalLeafCountBN
            )
            .accounts({ payer: user.publicKey, root: setupResult.rootPda })
            .rpc(),
          expectedError: "Invalid transaction hash",
        });
      });

      it("Should fail if invalid proof", async () => {
        const badProof = structuredClone(setupResult.proof);
        badProof.pop();

        await shouldFail({
          fn: program.methods
            .proveTransaction(
              transactionHash,
              nonce,
              REMOTE_MESSENGER,
              [messengerIxParam],
              badProof,
              setupResult.leafIndexBN,
              setupResult.totalLeafCountBN
            )
            .accounts({ payer: user.publicKey, root: setupResult.rootPda })
            .rpc(),
          expectedError: "Invalid proof",
        });
      });

      it("Posts output root", async () => {
        const tx = await program.methods
          .proveTransaction(
            transactionHash,
            nonce,
            REMOTE_MESSENGER,
            [messengerIxParam],
            setupResult.proof,
            setupResult.leafIndexBN,
            setupResult.totalLeafCountBN
          )
          .accounts({ payer: user.publicKey, root: setupResult.rootPda })
          .rpc();

        await confirmTransaction({ connection: provider.connection, tx });

        const message = await program.account.message.fetch(
          setupResult.messagePda
        );

        expect(message.ixs).to.eql([messengerIxParam]);
        expect(message.isExecuted).to.be.false;
      });
    });

    describe("Finalize transaction", () => {
      let userBalanceBefore: number;

      before(async () => {
        userBalanceBefore = Number(
          (await provider.connection.getTokenAccountBalance(userTokenAccount))
            .value.amount
        );

        const tx = await program.methods
          .finalizeTransaction()
          .accounts({
            message: setupResult.messagePda,
          })
          .remainingAccounts(finalizeSplBridgeAccounts)
          .rpc();

        await confirmTransaction({ connection: provider.connection, tx });
      });

      it("Should execute transaction", async () => {
        const message = await program.account.message.fetch(
          setupResult.messagePda
        );
        expect(message.isExecuted).to.be.true;
      });

      it("Should mark message successful", async () => {
        const message = await program.account.message.fetch(
          setupResult.messagePda
        );
        expect(message.successfulMessage).to.be.true;
      });

      it("Should transfer SPL tokens back to user", async () => {
        const userBalanceAfter = Number(
          (await provider.connection.getTokenAccountBalance(userTokenAccount))
            .value.amount
        );

        expect(userBalanceAfter).to.be.equal(
          userBalanceBefore + TRANSFER_AMOUNT
        );
      });

      it("Should fail if already executed", async () => {
        await shouldFail({
          fn: program.methods
            .finalizeTransaction()
            .accounts({
              message: setupResult.messagePda,
            })
            .remainingAccounts(finalizeSplBridgeAccounts)
            .rpc(),
          expectedError: "Already executed",
        });
      });
    });
  });

  describe("Returned SPL bridge transaction", () => {
    const FROM = Array.from({ length: 20 }, (_, i) => i);

    const [mintPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from(MINT_SEED),
        Buffer.from(REMOTE_TOKEN_ADDRESS),
        new anchor.BN(DECIMALS).toBuffer("le", 1),
      ],
      program.programId
    );

    const userTokenAccount = getAssociatedTokenAddressSync(
      mintPda,
      user.publicKey,
      false
    );

    const transferAccounts = [
      { pubkey: userTokenAccount, isWritable: true, isSigner: false },
      { pubkey: mintPda, isWritable: true, isSigner: false },
      { pubkey: TOKEN_PROGRAM_ID, isWritable: false, isSigner: false },
    ];

    const bridgeIxParam = createBridgeIxParam({
      localToken: mintPda,
      remoteToken: REMOTE_TOKEN_ADDRESS,
      from: FROM,
      to: userTokenAccount,
      amount: new anchor.BN(TRANSFER_AMOUNT),
      extraData: DUMMY_DATA,
    });

    const nonce = Array.from(Buffer.from(new anchor.BN(0).toArray("be", 32)));
    const messengerIxParam = createMessengerIxParam({
      nonce,
      sender: REMOTE_BRIDGE,
      ixParam: bridgeIxParam,
    });

    const transactionHash = hashIxs({
      nonce,
      remoteSender: REMOTE_MESSENGER,
      ixs: [messengerIxParam],
    });

    let setupResult: SetupRootAndProofResult;

    before(async () => {
      // const tx = await program.methods
      //   .createMint(REMOTE_TOKEN_ADDRESS, DECIMALS)
      //   .accounts({ tokenProgram: TOKEN_PROGRAM_ID })
      //   .rpc();
      // await confirmTransaction({ connection: provider.connection, tx });

      await createAssociatedTokenAccountIdempotent(
        provider.connection,
        user.payer,
        mintPda,
        user.publicKey
      );

      const blockNumber = new anchor.BN(24);
      setupResult = await setupRootAndProof({
        program,
        blockNumber,
        transactionHash,
      });
    });

    describe("Prove transction", () => {
      it("Should fail if invalid transaction hash", async () => {
        await shouldFail({
          fn: program.methods
            .proveTransaction(
              setupResult.transaction2,
              nonce,
              REMOTE_MESSENGER,
              [messengerIxParam],
              setupResult.proof,
              setupResult.leafIndexBN,
              setupResult.totalLeafCountBN
            )
            .accounts({ payer: user.publicKey, root: setupResult.rootPda })
            .rpc(),
          expectedError: "Invalid transaction hash",
        });
      });

      it("Should fail if invalid proof", async () => {
        const badProof = structuredClone(setupResult.proof);
        badProof.pop();

        await shouldFail({
          fn: program.methods
            .proveTransaction(
              transactionHash,
              nonce,
              REMOTE_MESSENGER,
              [messengerIxParam],
              badProof,
              setupResult.leafIndexBN,
              setupResult.totalLeafCountBN
            )
            .accounts({ payer: user.publicKey, root: setupResult.rootPda })
            .rpc(),
          expectedError: "Invalid proof",
        });
      });

      it("Posts output root", async () => {
        const tx = await program.methods
          .proveTransaction(
            transactionHash,
            nonce,
            REMOTE_MESSENGER,
            [messengerIxParam],
            setupResult.proof,
            setupResult.leafIndexBN,
            setupResult.totalLeafCountBN
          )
          .accounts({ payer: user.publicKey, root: setupResult.rootPda })
          .rpc();

        await confirmTransaction({ connection: provider.connection, tx });

        const message = await program.account.message.fetch(
          setupResult.messagePda
        );

        expect(message.ixs).to.eql([messengerIxParam]);
        expect(message.isExecuted).to.be.false;
      });
    });

    describe("Finalize transaction", () => {
      before(async () => {
        const tx = await program.methods
          .finalizeTransaction()
          .accounts({
            message: setupResult.messagePda,
          })
          .remainingAccounts(transferAccounts)
          .rpc();

        await confirmTransaction({ connection: provider.connection, tx });

        await printLogs({ connection: provider.connection, tx });
      });

      it("Should execute transaction", async () => {
        const message = await program.account.message.fetch(
          setupResult.messagePda
        );
        expect(message.isExecuted).to.be.true;
      });

      it("Should mark message successful", async () => {
        const message = await program.account.message.fetch(
          setupResult.messagePda
        );
        expect(message.successfulMessage).to.be.true;
      });

      it("Should fail if already executed", async () => {
        await shouldFail({
          fn: program.methods
            .finalizeTransaction()
            .accounts({
              message: setupResult.messagePda,
            })
            .remainingAccounts(transferAccounts)
            .rpc(),
          expectedError: "Already executed",
        });
      });
    });
  });
});
