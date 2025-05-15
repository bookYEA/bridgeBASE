import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Bridge } from "../../target/types/bridge";
import { expect } from "chai";
import { confirmTransaction } from "../utils/confirmTransaction";
import { PublicKey } from "@solana/web3.js";
import { fundAccount } from "../utils/fundAccount";
import { oracleSecretKey } from "../utils/constants";

describe("post root", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;
  const oracle = anchor.web3.Keypair.fromSecretKey(oracleSecretKey);

  let root = new Uint8Array(new Array(32).fill(0)) as any;
  const blockNumber = new anchor.BN(10);

  const [rootPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("output_root"), blockNumber.toBuffer("le", 8)],
    program.programId
  );

  before(async () => {
    await fundAccount(provider, provider.wallet.publicKey, oracle.publicKey);
  });

  beforeEach(() => {
    crypto.getRandomValues(root);
    root = [...root];
  });

  it("Posts output root", async () => {
    const tx = await program.methods
      .submitRoot(root as unknown as number[], blockNumber)
      .accounts({ payer: oracle.publicKey })
      .signers([oracle])
      .rpc();

    await confirmTransaction(provider.connection, tx);

    const outputRoot = await program.account.outputRoot.fetch(rootPda);

    expect(outputRoot.root).to.deep.equal(root);
    expect(outputRoot.blockNumber.eq(blockNumber)).to.be.true;
  });
});
