import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Bridge } from "../../target/types/bridge";
import { expect } from "chai";
import { getOpaqueData } from "../utils/getOpaqueData";
import { dummyData, toAddress } from "../utils/constants";

describe("portal", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;
  const user = provider.wallet as anchor.Wallet;

  const gasLimit = new anchor.BN(100000);
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
            .depositTransaction(toAddress, gasLimit, isCreation, dummyData)
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
        } catch (e) {
          _reject(e);
        }
      }
    );

    const expectedOpaqueData = getOpaqueData(gasLimit, isCreation, dummyData);

    await program.removeEventListener(listener);

    expect(slot).to.be.gt(0);
    expect(event.from.equals(user.publicKey)).to.be.true;
    expect(event.to).to.deep.equal(toAddress);
    expect(event.version.eq(new anchor.BN(0))).to.be.true;
    expect(Buffer.from(event.opaqueData)).to.eql(expectedOpaqueData);
  });
});
