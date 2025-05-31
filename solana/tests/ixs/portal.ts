import { expect } from "chai";
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";

import type { Bridge } from "../../target/types/bridge";

import { getOpaqueData } from "../utils/opaqueData";
import { DUMMY_DATA, programConstant } from "../utils/constants";
import { executeWithEventListener } from "../utils/confirmTransaction";

describe("portal", () => {
  // Common test setup
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Bridge as Program<Bridge>;
  const user = provider.wallet as anchor.Wallet;

  // Test constants
  const TO = Array.from({ length: 20 }, (_, i) => i);
  const GAS_LIMIT = new anchor.BN(100000);
  const IS_CREATION = false;

  // Program constants
  const GAS_FEE_RECEIVER = programConstant("gasFeeReceiver");
  const SOL_TO_ETH_FACTOR = programConstant("solToEthFactor");

  it("Deposits a transaction and emits an event", async () => {
    const { event, slot } = await executeWithEventListener({
      program,
      provider,
      transactionFn: () =>
        program.methods
          .depositTransaction(TO, GAS_LIMIT, IS_CREATION, DUMMY_DATA)
          .accounts({ user: user.publicKey })
          .rpc(),
    });

    const expectedOpaqueData = getOpaqueData({
      gasLimit: GAS_LIMIT,
      isCreation: IS_CREATION,
      data: DUMMY_DATA,
    });

    expect(slot).to.be.gt(0);
    expect(event.from.equals(user.publicKey)).to.be.true;
    expect(event.to).to.deep.equal(TO);
    expect(event.version.eq(new anchor.BN(0))).to.be.true;
    expect(Buffer.from(event.opaqueData)).to.eql(expectedOpaqueData);
  });

  it("Transfers gas fee to gas fee receiver", async () => {
    // Get gas fee receiver balance before transaction
    const balanceBefore =
      await provider.connection.getBalance(GAS_FEE_RECEIVER);

    // Execute deposit transaction
    const tx = await program.methods
      .depositTransaction(TO, GAS_LIMIT, IS_CREATION, DUMMY_DATA)
      .accounts({ user: user.publicKey })
      .rpc();

    const latestBlockHash = await provider.connection.getLatestBlockhash();
    await provider.connection.confirmTransaction(
      {
        blockhash: latestBlockHash.blockhash,
        lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
        signature: tx,
      },
      "confirmed"
    );

    // Get gas fee receiver balance after transaction
    const balanceAfter = await provider.connection.getBalance(GAS_FEE_RECEIVER);

    const baseFee = 30; // TODO: Change once we implement proper fee computation
    const expectedGasCost = GAS_LIMIT.toNumber() * baseFee * SOL_TO_ETH_FACTOR;

    // Verify the balance increased by the expected amount
    expect(balanceAfter).to.equal(balanceBefore + expectedGasCost);
  });
});
