import { expect } from "chai";
import { PublicKey } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";

import { Bridge } from "../../target/types/bridge";

import { confirmTransaction } from "../utils/confirmTransaction";
import { fundAccount } from "../utils/fundAccount";
import { programConstant, ORACLE_SECRET_KEY } from "../utils/constants";

describe("post root", () => {
  // Common test setup
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Bridge as Program<Bridge>;

  // Test constants
  const ORACLE = anchor.web3.Keypair.fromSecretKey(ORACLE_SECRET_KEY);
  const BLOCK_NUMBER = new anchor.BN(10);

  // Program constants
  const OUTPUT_ROOT_SEED = programConstant("outputRootSeed");

  // PDAs
  const [rootPda] = PublicKey.findProgramAddressSync(
    [Buffer.from(OUTPUT_ROOT_SEED), BLOCK_NUMBER.toBuffer("le", 8)],
    program.programId
  );

  let root = new Uint8Array(new Array(32).fill(0)) as any;

  before(async () => {
    await fundAccount({
      provider,
      from: provider.wallet.publicKey,
      to: ORACLE.publicKey,
    });
  });

  beforeEach(() => {
    crypto.getRandomValues(root);
    root = [...root];
  });

  it("Posts output root", async () => {
    const tx = await program.methods
      .submitRoot(root as unknown as number[], BLOCK_NUMBER)
      .accounts({ payer: ORACLE.publicKey })
      .signers([ORACLE])
      .rpc();

    await confirmTransaction({ connection: provider.connection, tx });

    const outputRoot = await program.account.outputRoot.fetch(rootPda);

    expect(outputRoot.root).to.deep.equal(root);
    expect(outputRoot.blockNumber.eq(BLOCK_NUMBER)).to.be.true;
  });
});
