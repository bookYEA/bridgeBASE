import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Bridge } from "../../target/types/bridge";
import { expect } from "chai";
import { PublicKey } from "@solana/web3.js";
import {
  createMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  TOKEN_PROGRAM_ID,
  Account,
  getAssociatedTokenAddress,
} from "@solana/spl-token";
import {
  dummyData,
  expectedMessengerPubkey,
  minGasLimit,
  otherMessengerAddress,
  toAddress,
} from "../utils/constants";
import { getOpaqueDataFromBridge } from "../utils/getOpaqueData";
import { confirmTransaction } from "../utils/confirmTransaction";

describe("standard bridge", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;
  const user = provider.wallet as anchor.Wallet;

  const solLocalAddress = new PublicKey(
    Buffer.from(
      "0501550155015501550155015501550155015501550155015501550155015501",
      "hex"
    )
  );
  const solRemoteAddress = Uint8Array.from(
    Buffer.from("E398D7afe84A6339783718935087a4AcE6F6DFE8", "hex")
  ) as unknown as number[]; // random address for testing

  const value = new anchor.BN(1 * anchor.web3.LAMPORTS_PER_SOL); // 1 SOL

  // Find the vault PDA
  const [vaultPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("bridge_vault")],
    program.programId
  );

  const mintAuthSC = anchor.web3.Keypair.generate();
  const mintKeypairSC = anchor.web3.Keypair.generate();
  let mintSC: PublicKey;
  let userATA: Account;

  before(async () => {
    mintSC = await createMint(
      provider.connection,
      user.payer,
      mintAuthSC.publicKey,
      mintAuthSC.publicKey,
      10,
      mintKeypairSC,
      undefined,
      TOKEN_PROGRAM_ID
    );
    userATA = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      user.payer,
      mintSC,
      user.publicKey
    );
    await mintTo(
      provider.connection,
      user.payer,
      mintSC,
      userATA.address,
      mintAuthSC,
      100 * anchor.web3.LAMPORTS_PER_SOL,
      [],
      undefined,
      TOKEN_PROGRAM_ID
    );
  });

  describe("SOL Bridging", () => {
    it("Deposits a transaction and emits an event", async () => {
      let listener = null;
      let [event, slot]: [any, number] = await new Promise(
        async (resolve, reject) => {
          listener = program.addEventListener(
            "transactionDeposited",
            (event, slot) => {
              resolve([event, slot]);
            }
          );

          try {
            const tx = await program.methods
              .bridgeSolTo(
                solRemoteAddress,
                toAddress,
                value,
                minGasLimit,
                dummyData
              )
              .accounts({ user: user.publicKey })
              .rpc();

            await confirmTransaction(provider.connection, tx);
          } catch (e) {
            reject(e);
          }
        }
      );

      const expectedOpaqueData = await getOpaqueDataFromBridge(
        program,
        solRemoteAddress,
        solLocalAddress,
        toAddress,
        value,
        dummyData,
        user.publicKey,
        minGasLimit
      );

      await program.removeEventListener(listener);

      expect(slot).to.be.gt(0);
      expect(event.from.equals(expectedMessengerPubkey)).to.be.true;
      expect(event.to).to.deep.equal(otherMessengerAddress);
      expect(event.version.eq(new anchor.BN(0))).to.be.true;
      expect(Buffer.from(event.opaqueData)).to.eql(expectedOpaqueData);
    });

    it("transfers lamports to vault", async () => {
      const vaultAccountInfo = await provider.connection.getAccountInfo(
        vaultPda
      );
      const vaultBalanceBefore = vaultAccountInfo?.lamports ?? 0;

      await program.methods
        .bridgeSolTo(solRemoteAddress, toAddress, value, minGasLimit, dummyData)
        .accounts({ user: user.publicKey })
        .rpc();

      const vaultAccountInfoAfter = await provider.connection.getAccountInfo(
        vaultPda
      );
      const vaultBalanceAfter = vaultAccountInfoAfter?.lamports ?? 0;

      expect(vaultBalanceAfter).to.equal(vaultBalanceBefore + value.toNumber());
    });

    it("transfers lamports from user", async () => {
      const userAccountInfo = await provider.connection.getAccountInfo(
        user.publicKey
      );
      const vaultBalanceBefore = userAccountInfo?.lamports ?? 0;

      await program.methods
        .bridgeSolTo(solRemoteAddress, toAddress, value, minGasLimit, dummyData)
        .accounts({ user: user.publicKey })
        .rpc();

      const userAccountInfoAfter = await provider.connection.getAccountInfo(
        user.publicKey
      );
      const userBalanceAfter = userAccountInfoAfter?.lamports ?? 0;

      expect(userBalanceAfter).to.be.below(
        vaultBalanceBefore - value.toNumber()
      );
    });
  });

  describe("SPL Bridging", () => {
    it("Deposits a transaction and emits an event", async () => {
      let listener = null;
      let [event, slot]: [any, number] = await new Promise(
        async (resolve, reject) => {
          listener = program.addEventListener(
            "transactionDeposited",
            (event, slot) => {
              resolve([event, slot]);
            }
          );

          try {
            const tx = await program.methods
              .bridgeTokensTo(
                solRemoteAddress,
                toAddress,
                value,
                minGasLimit,
                dummyData
              )
              .accounts({
                user: user.publicKey,
                mint: mintSC,
              })
              .rpc();

            await confirmTransaction(provider.connection, tx);
          } catch (e) {
            reject(e);
          }
        }
      );

      const expectedOpaqueData = await getOpaqueDataFromBridge(
        program,
        solRemoteAddress,
        mintSC,
        toAddress,
        value,
        dummyData,
        user.publicKey,
        minGasLimit
      );

      await program.removeEventListener(listener);

      expect(slot).to.be.gt(0);
      expect(event.from.equals(expectedMessengerPubkey)).to.be.true;
      expect(event.to).to.deep.equal(otherMessengerAddress);
      expect(event.version.eq(new anchor.BN(0))).to.be.true;
      expect(Buffer.from(event.opaqueData)).to.eql(expectedOpaqueData);
    });

    it("transfers tokens to vault", async () => {
      const tokenAccount = await getAssociatedTokenAddress(
        mintSC,
        vaultPda,
        true
      );

      const vaultBalanceBeforeRes =
        await provider.connection.getTokenAccountBalance(tokenAccount);
      const vaultBalanceBefore = Number(vaultBalanceBeforeRes.value.amount);

      await program.methods
        .bridgeTokensTo(
          solRemoteAddress,
          toAddress,
          value,
          minGasLimit,
          dummyData
        )
        .accounts({ user: user.publicKey, mint: mintSC })
        .rpc();

      // Get vault balance after
      const vaultBalanceAfterRes =
        await provider.connection.getTokenAccountBalance(tokenAccount);
      const vaultBalanceAfter = Number(vaultBalanceAfterRes.value.amount);

      expect(vaultBalanceAfter).to.equal(vaultBalanceBefore + value.toNumber());
    });

    it("transfers tokens from user", async () => {
      const tokenAccount = await getAssociatedTokenAddress(
        mintSC,
        user.publicKey,
        true
      );

      const userBalanceBeforeRes =
        await provider.connection.getTokenAccountBalance(tokenAccount);
      const userBalanceBefore = Number(userBalanceBeforeRes.value.amount);

      await program.methods
        .bridgeTokensTo(
          solRemoteAddress,
          toAddress,
          value,
          minGasLimit,
          dummyData
        )
        .accounts({ user: user.publicKey, mint: mintSC })
        .rpc();

      // Get user balance after
      const userBalanceAfterRes =
        await provider.connection.getTokenAccountBalance(tokenAccount);
      const userBalanceAfter = Number(userBalanceAfterRes.value.amount);

      expect(userBalanceAfter).to.equal(userBalanceBefore - value.toNumber());
    });
  });
});
