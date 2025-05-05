import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Bridge } from "../../target/types/bridge";
import { expect } from "chai";
import { PublicKey } from "@solana/web3.js";

describe("portal", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;
  const user = provider.wallet as anchor.Wallet;

  // Generate a dummy EVM address (20 bytes)
  const dummyEvmAddress = Array.from({ length: 20 }, (_, i) => i);
  const value = new anchor.BN(1 * anchor.web3.LAMPORTS_PER_SOL); // 1 SOL
  const gasLimit = new anchor.BN(100000);
  const data = Buffer.from("sample data payload", "utf-8");
  const isCreation = false;

  // Find the vault PDA
  const [vaultPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("bridge_vault")],
    program.programId
  );

  it("Deposits a transaction and emits an event", async () => {
    let listener = null;
    let [event, slot]: [any, number] = await new Promise(
      async (resolve, _reject) => {
        listener = program.addEventListener(
          "transactionDeposited",
          (event, slot) => {
            resolve([event, slot]);
          }
        );

        const tx = await program.methods
          .depositTransaction(
            dummyEvmAddress,
            value,
            gasLimit,
            isCreation,
            data
          )
          .accounts({ user: user.publicKey })
          .rpc();

        console.log("Deposit transaction signature", tx);
        const latestBlockHash = await provider.connection.getLatestBlockhash();
        await provider.connection.confirmTransaction(
          {
            blockhash: latestBlockHash.blockhash,
            lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
            signature: tx,
          },
          "confirmed"
        );

        // Logs can be helpful for debugging but removed for brevity here
        // const txDetails = await provider.connection.getTransaction(tx, {
        //   maxSupportedTransactionVersion: 0,
        //   commitment: "confirmed",
        // });
        // const logs = txDetails?.meta?.logMessages || null;
        // console.log(logs);
      }
    );

    // abi.encodePacked(value, value, gasLimit, isCreation, data)
    const expectedOpaqueData = Buffer.concat([
      Buffer.from(value.toArray("be", 8)), // msg_value (8 bytes, big-endian)
      Buffer.from(value.toArray("be", 8)), // value (8 bytes, big-endian)
      Buffer.from(gasLimit.toArray("be", 8)), // gas_limit (8 bytes, big-endian)
      Buffer.from([isCreation ? 1 : 0]), // is_creation (1 byte)
      data, // data payload
    ]);

    await program.removeEventListener(listener);

    expect(slot).to.be.gt(0);
    expect(event.from.equals(user.publicKey)).to.be.true;
    expect(event.to).to.deep.equal(dummyEvmAddress);
    expect(event.version.eq(new anchor.BN(0))).to.be.true;
    expect(Buffer.from(event.opaqueData)).to.eql(expectedOpaqueData);
  });

  it("transfers lamports to vault", async () => {
    const vaultAccountInfo = await provider.connection.getAccountInfo(vaultPda);
    const vaultBalanceBefore = vaultAccountInfo?.lamports ?? 0;

    await program.methods
      .depositTransaction(dummyEvmAddress, value, gasLimit, isCreation, data)
      .accounts({ user: user.publicKey })
      .rpc();

    // Get vault balance after
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
      .depositTransaction(dummyEvmAddress, value, gasLimit, isCreation, data)
      .accounts({ user: user.publicKey })
      .rpc();

    // Get vault balance after
    const userAccountInfoAfter = await provider.connection.getAccountInfo(
      user.publicKey
    );
    const userBalanceAfter = userAccountInfoAfter?.lamports ?? 0;

    expect(userBalanceAfter).to.be.below(vaultBalanceBefore - value.toNumber());
  });
});
