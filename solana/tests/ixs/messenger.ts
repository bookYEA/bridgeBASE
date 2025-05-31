import { expect } from "chai";
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";

import { Bridge } from "../../target/types/bridge";

import { DUMMY_DATA, MIN_GAS_LIMIT, programConstant } from "../utils/constants";
import { getOpaqueDataFromMessenger } from "../utils/opaqueData";
import { executeWithEventListener } from "../utils/confirmTransaction";
import { virtualPubkey } from "../utils/virtualPubkey";

describe("messenger", () => {
  // Common test setup
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Bridge as Program<Bridge>;
  const user = provider.wallet as anchor.Wallet;

  // Test constants
  const TARGET = Array.from({ length: 20 }, (_, i) => i);

  // Program constants
  const REMOTE_MESSENGER = programConstant("remoteMessenger");

  before(async () => {
    await program.methods.initialize().accounts({ user: user.publicKey }).rpc();
  });

  it("Deposits a transaction and emits an event", async () => {
    const { event, slot } = await executeWithEventListener({
      program,
      provider,
      transactionFn: () =>
        program.methods
          .sendMessage(TARGET, DUMMY_DATA, MIN_GAS_LIMIT)
          .accounts({ user: user.publicKey })
          .rpc(),
    });

    const expectedOpaqueData = await getOpaqueDataFromMessenger({
      program,
      extraData: DUMMY_DATA,
      sender: user.publicKey,
      toAddress: TARGET,
      minGasLimit: MIN_GAS_LIMIT,
    });

    const messengerVirtualPubkey = virtualPubkey("messenger");

    expect(slot).to.be.gt(0);
    expect(event.from.equals(messengerVirtualPubkey)).to.be.true;
    expect(event.to).to.deep.equal(REMOTE_MESSENGER);
    expect(event.version.eq(new anchor.BN(0))).to.be.true;
    expect(Buffer.from(event.opaqueData)).to.eql(expectedOpaqueData);
  });
});
