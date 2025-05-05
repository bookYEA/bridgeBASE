import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Bridge } from "../../target/types/bridge";
import { expect } from "chai";

describe("portal", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;
  const user = provider.wallet as anchor.Wallet;

  // Generate a dummy EVM address (20 bytes)
  const toAddress = Array.from({ length: 20 }, (_, i) => i);
  const gasLimit = new anchor.BN(100000);
  const data = Buffer.from("sample data payload", "utf-8");
  const isCreation = false;

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

        try {
          const tx = await program.methods
            .depositTransaction(toAddress, gasLimit, isCreation, data)
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
          _reject(e);
        }
      }
    );

    // abi.encodePacked(gasLimit, isCreation, data)
    const expectedOpaqueData = Buffer.concat([
      Buffer.from(gasLimit.toArray("be", 8)), // gas_limit (8 bytes, big-endian)
      Buffer.from([isCreation ? 1 : 0]), // is_creation (1 byte)
      data, // data payload
    ]);

    await program.removeEventListener(listener);

    expect(slot).to.be.gt(0);
    expect(event.from.equals(user.publicKey)).to.be.true;
    expect(event.to).to.deep.equal(toAddress);
    expect(event.version.eq(new anchor.BN(0))).to.be.true;
    expect(Buffer.from(event.opaqueData)).to.eql(expectedOpaqueData);
  });
});
