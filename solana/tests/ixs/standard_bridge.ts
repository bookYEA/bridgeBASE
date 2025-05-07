import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Bridge } from "../../target/types/bridge";
import { expect } from "chai";
import { PublicKey } from "@solana/web3.js";

describe("standard bridge", () => {
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
  const expectedBridgePubkey = new PublicKey(
    Buffer.from(
      "7a25452c36304317d6fe970091c383b0d45e9b0b06485d2561156f025c6936af",
      "hex"
    )
  );

  const solLocalAddress = new PublicKey(
    Buffer.from(
      "0501550155015501550155015501550155015501550155015501550155015501",
      "hex"
    )
  );
  const solRemoteAddress = Uint8Array.from(
    Buffer.from("E398D7afe84A6339783718935087a4AcE6F6DFE8", "hex")
  ) as unknown as number[]; // random address for testing

  // Generate a dummy EVM address (20 bytes)
  const toAddress = Array.from({ length: 20 }, (_, i) => i);
  const otherMessengerAddress = [
    248, 66, 18, 131, 56, 6, 186, 55, 37, 119, 129, 17, 124, 17, 145, 8, 242,
    20, 80, 9,
  ];
  const otherBridgeAddress = [
    184, 148, 125, 39, 37, 211, 233, 222, 155, 25, 252, 114, 15, 5, 51, 0, 197,
    9, 129, 229,
  ];
  const extraData = Buffer.from("sample data payload", "utf-8");
  const value = new anchor.BN(1 * anchor.web3.LAMPORTS_PER_SOL); // 1 SOL
  const minGasLimit = 100000;

  // Find the vault PDA
  const [vaultPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("bridge_vault")],
    program.programId
  );
  const [messengerPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("messenger_state")],
    program.programId
  );

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
              solLocalAddress,
              solRemoteAddress,
              toAddress,
              value,
              minGasLimit,
              extraData
            )
            .accounts({ user: user.publicKey })
            .rpc();

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

    const extraDataPaddingBytes = 32 - (extraData.length % 32);
    const message = Buffer.concat([
      Buffer.from("2d916920", "hex"), // function selector
      Buffer.from([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, ...solRemoteAddress]), // remote token
      solLocalAddress.toBuffer(), // local token
      user.publicKey.toBuffer(), // from
      Buffer.from([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, ...toAddress]), // target
      Buffer.from(value.toArray("be", 32)), // value
      Buffer.from(Array.from({ length: 32 }, (_, i) => (i == 31 ? 192 : 0))), // extra_data offset
      Buffer.from(
        new anchor.BN(Buffer.from(extraData).length).toArray("be", 32)
      ),
      extraData,
      Buffer.from(Array.from({ length: extraDataPaddingBytes }, () => 0)),
    ]);

    const messenger = await program.account.messenger.fetch(messengerPda);

    const paddingBytes = 32 - (message.length % 32);
    const data = Buffer.concat([
      Buffer.from([84, 170, 67, 163]), // function selector
      Buffer.from(
        Array.from({ length: 32 }, (_, i) => {
          if (i === 1) {
            return 1;
          } else if (i === 31) {
            return messenger.msgNonce.toNumber() - 1;
          }
          return 0;
        })
      ), // nonce
      expectedBridgePubkey.toBuffer(), // sender
      Buffer.from([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, ...otherBridgeAddress]), // target
      Buffer.from(new anchor.BN(0).toArray("be", 32)), // value
      Buffer.from(new anchor.BN(minGasLimit).toArray("be", 32)), // min gas
      Buffer.from(Array.from({ length: 32 }, (_, i) => (i == 31 ? 192 : 0))), // message offset
      Buffer.from(new anchor.BN(Buffer.from(message).length).toArray("be", 32)), // message length
      message, // message
      Buffer.from(Array.from({ length: paddingBytes }, () => 0)), // ensure message is multiple of 32 bytes
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

    // abi.encodePacked(gasLimit, isCreation, data)
    const expectedOpaqueData = Buffer.concat([
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
      .bridgeTokensTo(
        solLocalAddress,
        solRemoteAddress,
        toAddress,
        value,
        minGasLimit,
        extraData
      )
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
      .bridgeTokensTo(
        solLocalAddress,
        solRemoteAddress,
        toAddress,
        value,
        minGasLimit,
        extraData
      )
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
