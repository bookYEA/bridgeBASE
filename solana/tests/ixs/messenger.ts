import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Bridge } from "../../target/types/bridge";
import { expect } from "chai";
import {
  dummyData,
  expectedMessengerPubkey,
  minGasLimit,
  otherMessengerAddress,
  toAddress,
} from "../utils/constants";
import { getOpaqueDataFromMessenger } from "../utils/getOpaqueData";

describe("messenger", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;
  const user = provider.wallet as anchor.Wallet;

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
            .sendMessage(toAddress, dummyData, minGasLimit)
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
          reject(e);
        }
      }
    );

    const expectedOpaqueData = await getOpaqueDataFromMessenger(
      program,
      dummyData,
      user.publicKey,
      toAddress,
      minGasLimit
    );

    await program.removeEventListener(listener);

    expect(slot).to.be.gt(0);
    expect(event.from.equals(expectedMessengerPubkey)).to.be.true;
    expect(event.to).to.deep.equal(otherMessengerAddress);
    expect(event.version.eq(new anchor.BN(0))).to.be.true;
    expect(Buffer.from(event.opaqueData)).to.eql(expectedOpaqueData);
  });
});
