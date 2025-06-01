import { expect } from "chai";
import { PublicKey } from "@solana/web3.js";
import {
  createMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
} from "@solana/spl-token";
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";

import { Bridge } from "../../target/types/bridge";

import {
  DUMMY_DATA,
  MIN_GAS_LIMIT,
  programConstant,
  REMOTE_TOKEN_ADDRESS,
} from "../utils/constants";
import { getOpaqueDataFromBridge } from "../utils/opaqueData";
import { executeWithEventListener } from "../utils/confirmTransaction";
import { virtualPubkey } from "../utils/virtualPubkey";

describe("standard bridge", () => {
  // Common test setup
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Bridge as Program<Bridge>;
  const user = provider.wallet as anchor.Wallet;

  // Test constants
  const TO = Array.from({ length: 20 }, (_, i) => i);

  // Program constants
  const SOL_VAULT_SEED = programConstant("solVaultSeed");
  const REMOTE_MESSENGER_SEED = programConstant("remoteMessenger");
  const NATIVE_SOL_PUBKEY = programConstant("nativeSolPubkey");
  const TOKEN_VAULT_SEED = programConstant("tokenVaultSeed");

  // PDAs
  const [solVault] = PublicKey.findProgramAddressSync(
    [Buffer.from(SOL_VAULT_SEED), Buffer.from(REMOTE_TOKEN_ADDRESS)],
    program.programId
  );

  const virtualMessengerPubkey = virtualPubkey("messenger");

  before(async () => {
    // Already initialized by a previous test
    // await program.methods.initialize().accounts({ user: user.publicKey }).rpc();
  });

  describe("SOL Bridging", () => {
    const TRANSFER_AMOUNT = new anchor.BN(5 * anchor.web3.LAMPORTS_PER_SOL);

    it("Deposits a transaction and emits an event", async () => {
      const { event, slot } = await executeWithEventListener({
        program,
        provider,
        transactionFn: () =>
          program.methods
            .bridgeSolTo(
              REMOTE_TOKEN_ADDRESS,
              TO,
              TRANSFER_AMOUNT,
              MIN_GAS_LIMIT,
              DUMMY_DATA
            )
            .accounts({ user: user.publicKey })
            .rpc(),
      });

      const expectedOpaqueData = await getOpaqueDataFromBridge({
        program,
        remoteToken: REMOTE_TOKEN_ADDRESS,
        localToken: NATIVE_SOL_PUBKEY,
        toAddress: TO,
        value: TRANSFER_AMOUNT,
        extraData: DUMMY_DATA,
        sender: user.publicKey,
        minGasLimit: MIN_GAS_LIMIT,
      });

      expect(slot).to.be.gt(0);
      expect(event.from.equals(virtualMessengerPubkey)).to.be.true;
      expect(event.to).to.deep.equal(REMOTE_MESSENGER_SEED);
      expect(event.version.eq(new anchor.BN(0))).to.be.true;
      expect(Buffer.from(event.opaqueData)).to.eql(expectedOpaqueData);
    });

    it("Transfers lamports from user to vault", async () => {
      const userAccountInfo = await provider.connection.getAccountInfo(
        user.publicKey
      );

      const solVaultAccountInfo =
        await provider.connection.getAccountInfo(solVault);

      const userBalanceBefore = userAccountInfo?.lamports ?? 0;
      const vaultBalanceBefore = solVaultAccountInfo?.lamports ?? 0;

      await program.methods
        .bridgeSolTo(
          REMOTE_TOKEN_ADDRESS,
          TO,
          TRANSFER_AMOUNT,
          MIN_GAS_LIMIT,
          DUMMY_DATA
        )
        .accounts({ user: user.publicKey })
        .rpc();

      const userAccountInfoAfter = await provider.connection.getAccountInfo(
        user.publicKey
      );

      const vaultAccountInfoAfter =
        await provider.connection.getAccountInfo(solVault);

      const userBalanceAfter = userAccountInfoAfter?.lamports ?? 0;
      const vaultBalanceAfter = vaultAccountInfoAfter?.lamports ?? 0;

      expect(userBalanceAfter).to.be.below(
        userBalanceBefore - TRANSFER_AMOUNT.toNumber()
      );
      expect(vaultBalanceAfter).to.equal(
        vaultBalanceBefore + TRANSFER_AMOUNT.toNumber()
      );
    });
  });

  describe("SPL Bridging", () => {
    const mintAuth = anchor.web3.Keypair.generate();
    const mintKeypair = anchor.web3.Keypair.generate();
    const TRANSFER_AMOUNT = new anchor.BN(5 * anchor.web3.LAMPORTS_PER_SOL);

    let mint: PublicKey;
    let vaultTokenAccount: PublicKey;
    let userTokenAccount: PublicKey;

    before(async () => {
      mint = await createMint(
        provider.connection,
        user.payer,
        mintAuth.publicKey,
        mintAuth.publicKey,
        9,
        mintKeypair
      );

      [vaultTokenAccount] = PublicKey.findProgramAddressSync(
        [
          Buffer.from(TOKEN_VAULT_SEED),
          mint.toBuffer(),
          Buffer.from(REMOTE_TOKEN_ADDRESS),
        ],
        program.programId
      );

      userTokenAccount = (
        await getOrCreateAssociatedTokenAccount(
          provider.connection,
          user.payer,
          mint,
          user.publicKey
        )
      ).address;

      await mintTo(
        provider.connection,
        user.payer,
        mint,
        userTokenAccount,
        mintAuth,
        1000 * anchor.web3.LAMPORTS_PER_SOL
      );
    });

    it("Deposits a transaction and emits an event", async () => {
      const { event, slot } = await executeWithEventListener({
        program,
        provider,
        transactionFn: () =>
          program.methods
            .bridgeTokensTo(
              REMOTE_TOKEN_ADDRESS,
              TO,
              TRANSFER_AMOUNT,
              MIN_GAS_LIMIT,
              DUMMY_DATA
            )
            .accounts({
              user: user.publicKey,
              mint,
              fromTokenAccount: userTokenAccount,
            })
            .rpc(),
      });

      const expectedOpaqueData = await getOpaqueDataFromBridge({
        program,
        remoteToken: REMOTE_TOKEN_ADDRESS,
        localToken: mint,
        toAddress: TO,
        value: TRANSFER_AMOUNT,
        extraData: DUMMY_DATA,
        sender: user.publicKey,
        minGasLimit: MIN_GAS_LIMIT,
      });

      expect(slot).to.be.gt(0);
      expect(event.from.equals(virtualMessengerPubkey)).to.be.true;
      expect(event.to).to.deep.equal(REMOTE_MESSENGER_SEED);
      expect(event.version.eq(new anchor.BN(0))).to.be.true;
      expect(Buffer.from(event.opaqueData)).to.eql(expectedOpaqueData);
    });

    it("Transfers tokens from user to vault", async () => {
      const userBalanceBefore = Number(
        (await provider.connection.getTokenAccountBalance(userTokenAccount))
          .value.amount
      );

      const vaultBalanceBefore = Number(
        (await provider.connection.getTokenAccountBalance(vaultTokenAccount))
          .value.amount
      );

      await program.methods
        .bridgeTokensTo(
          REMOTE_TOKEN_ADDRESS,
          TO,
          TRANSFER_AMOUNT,
          MIN_GAS_LIMIT,
          DUMMY_DATA
        )
        .accounts({
          user: user.publicKey,
          mint,
          fromTokenAccount: userTokenAccount,
        })
        .rpc();

      const userBalanceAfter = Number(
        (await provider.connection.getTokenAccountBalance(userTokenAccount))
          .value.amount
      );

      const vaultBalanceAfter = Number(
        (await provider.connection.getTokenAccountBalance(vaultTokenAccount))
          .value.amount
      );

      expect(userBalanceAfter).to.equal(
        userBalanceBefore - TRANSFER_AMOUNT.toNumber()
      );

      expect(vaultBalanceAfter).to.equal(
        vaultBalanceBefore + TRANSFER_AMOUNT.toNumber()
      );
    });
  });

  // TODO: This test is hard to write because if requires mocking a token account with SPL token from the bridge.
  //       Mocking is not doable in TS afaik, we should move to rust.
  // describe("ERC20 bridging", () => {
  //   const MINT_SEED = programConstant("mintSeed");
  //   const DECIMALS = 9;
  //   const TRANSFER_AMOUNT = new anchor.BN(5 * anchor.web3.LAMPORTS_PER_SOL);

  //   const [mintPda] = PublicKey.findProgramAddressSync(
  //     [
  //       Buffer.from(MINT_SEED),
  //       Buffer.from(REMOTE_TOKEN_ADDRESS),
  //       new anchor.BN(DECIMALS).toBuffer("le", 1),
  //     ],
  //     program.programId
  //   );

  //   before(async () => {
  //     // Initialize the mint PDA for the remote token.
  //     const tx = await program.methods
  //       .createMint(REMOTE_TOKEN_ADDRESS, DECIMALS)
  //       .accounts({ tokenProgram: TOKEN_PROGRAM_ID })
  //       .rpc();

  //     await confirmTransaction({ connection: provider.connection, tx });

  //     // Initialize and fund the from token account.
  //     const fromTokenAccount = await createAccount(
  //       provider.connection,
  //       user.payer,
  //       mintPda,
  //       user.publicKey
  //     );
  //   });

  //   it("Deposits a transaction and emits an event", async () => {
  //     let listener = null;
  //     let [event, slot]: [any, number] = await new Promise(
  //       async (resolve, reject) => {
  //         listener = program.addEventListener(
  //           "transactionDeposited",
  //           (event, slot) => {
  //             resolve([event, slot]);
  //           }
  //         );

  //         try {
  //           const tx = await program.methods
  //             .bridgeTokensTo(
  //               REMOTE_TOKEN_ADDRESS,
  //               TO,
  //               TRANSFER_AMOUNT,
  //               minGasLimit,
  //               dummyData
  //             )
  //             .accounts({
  //               user: user.publicKey,
  //               mint: mintPda,
  //             })
  //             .rpc();

  //           await confirmTransaction({ connection: provider.connection, tx });
  //         } catch (e) {
  //           reject(e);
  //         }
  //       }
  //     );

  //     // const expectedOpaqueData = await getOpaqueDataFromBridge(
  //     //   program,
  //     //   remoteTokenAddress,
  //     //   mintPda,
  //     //   toAddress,
  //     //   value,
  //     //   dummyData,
  //     //   user.publicKey,
  //     //   minGasLimit
  //     // );

  //     // await program.removeEventListener(listener);

  //     // expect(slot).to.be.gt(0);
  //     // expect(event.from.equals(virtualMessengerPubkey)).to.be.true;
  //     // expect(event.to).to.deep.equal(otherMessengerAddress);
  //     // expect(event.version.eq(new anchor.BN(0))).to.be.true;
  //     // expect(Buffer.from(event.opaqueData)).to.eql(expectedOpaqueData);
  //   });

  //   // it("transfers tokens from user", async () => {
  //   //   const tokenAccount = await getAssociatedTokenAddress(
  //   //     mintPda,
  //   //     user.publicKey,
  //   //     true
  //   //   );

  //   //   const userBalanceBeforeRes =
  //   //     await provider.connection.getTokenAccountBalance(tokenAccount);
  //   //   const userBalanceBefore = Number(userBalanceBeforeRes.value.amount);

  //   //   await program.methods
  //   //     .bridgeTokensTo(
  //   //       remoteTokenAddress,
  //   //       toAddress,
  //   //       value,
  //   //       minGasLimit,
  //   //       dummyData
  //   //     )
  //   //     .accounts({ user: user.publicKey, mint: mintPda })
  //   //     .rpc();

  //   //   // Get user balance after
  //   //   const userBalanceAfterRes =
  //   //     await provider.connection.getTokenAccountBalance(tokenAccount);
  //   //   const userBalanceAfter = Number(userBalanceAfterRes.value.amount);

  //   //   expect(userBalanceAfter).to.equal(userBalanceBefore - value.toNumber());
  //   // });
  // });
});
