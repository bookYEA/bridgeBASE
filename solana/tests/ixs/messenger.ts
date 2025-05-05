import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Bridge } from "../../target/types/bridge";
import { expect } from "chai";
import { PublicKey } from "@solana/web3.js";

describe("messenger", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;
  const user = provider.wallet as anchor.Wallet;

  const expectedMessengerPubkey = new PublicKey(
    Buffer.from(
      "7e273983f136714ba93a740a050279b541d6f25ebc6bbc6fc67616d0d5529cea",
      "hex"
    )
  );

  // Generate a dummy EVM address (20 bytes)
  const dummyEvmAddress = Array.from({ length: 20 }, (_, i) => i);
  const otherMessengerAddress = [
    95, 241, 55, 212, 176, 253, 205, 73, 220, 163, 12, 124, 245, 126, 87, 138,
    2, 109, 39, 137,
  ];
  const message = Buffer.from("sample data payload", "utf-8");
  const value = new anchor.BN(1 * anchor.web3.LAMPORTS_PER_SOL); // 1 SOL
  const minGasLimit = 100000;

  // Find the vault PDA
  const [vaultPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("bridge_vault")],
    program.programId
  );

  before(async () => {
    await program.methods.initialize().accounts({ user: user.publicKey }).rpc();
  });

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
            .sendMessage(dummyEvmAddress, message, value, minGasLimit)
            .accounts({ user: user.publicKey })
            .rpc();

          console.log("Deposit transaction signature", tx);
          const latestBlockHash =
            await provider.connection.getLatestBlockhash();
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
        } catch (e) {
          reject(e);
        }
      }
    );

    const paddingBytes = 32 - (message.length % 32);
    const data = Buffer.concat([
      Buffer.from([215, 100, 173, 11]), // function selector
      Buffer.from(Array.from({ length: 32 }, (_, i) => (i === 1 ? 1 : 0))), // nonce
      user.publicKey.toBuffer(), // sender
      Buffer.from([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, ...dummyEvmAddress]), // target
      Buffer.from(value.toArray("be", 32)), // value
      Buffer.from(new anchor.BN(minGasLimit).toArray("be", 32)), // minGasLimit
      Buffer.from(Array.from({ length: 32 }, (_, i) => (i == 31 ? 192 : 0))), // message offset
      Buffer.from(new anchor.BN(Buffer.from(message).length).toArray("be", 32)), // message length
      message, // message
      Buffer.from(Array.from({ length: paddingBytes }, () => 0)), // padding to ensure message length is multiple of 32 bytes
    ]);
    const execution_gas =
      200_000 + 40_000 + 40_000 + 5_000 + (minGasLimit * 64) / 63;
    const total_message_size = message.length + 260;

    const gasLimit =
      21_000 +
      Math.max(
        execution_gas + total_message_size * 16,
        total_message_size * 40
      );

    // abi.encodePacked(value, value, gasLimit, isCreation, data)
    const expectedOpaqueData = Buffer.concat([
      Buffer.from(value.toArray("be", 8)), // msg_value (8 bytes, big-endian)
      Buffer.from(value.toArray("be", 8)), // value (8 bytes, big-endian)
      Buffer.from(new anchor.BN(gasLimit).toArray("be", 8)), // gas_limit (8 bytes, big-endian)
      Buffer.from([0]), // is_creation (1 byte)
      data, // data payload
    ]);

    await program.removeEventListener(listener);

    expect(slot).to.be.gt(0);
    expect(event.from.equals(expectedMessengerPubkey)).to.be.true;
    expect(event.to).to.deep.equal(otherMessengerAddress);
    expect(event.version.eq(new anchor.BN(0))).to.be.true;
    expect(Buffer.from(event.opaqueData)).to.eql(expectedOpaqueData);
  });

  it("transfers lamports to vault", async () => {
    const vaultAccountInfo = await provider.connection.getAccountInfo(vaultPda);
    const vaultBalanceBefore = vaultAccountInfo?.lamports ?? 0;

    await program.methods
      .sendMessage(dummyEvmAddress, message, value, minGasLimit)
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
      .sendMessage(dummyEvmAddress, message, value, minGasLimit)
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
