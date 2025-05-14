import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Bridge } from "../../target/types/bridge";
import { expect } from "chai";
import { confirmTransaction } from "../utils/confirmTransaction";
import { PublicKey } from "@solana/web3.js";
import { fundAccount } from "../utils/fundAccount";

describe("post root", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;
  const oracleSecretKey = Uint8Array.from([
    232, 74, 68, 137, 42, 170, 245, 110, 221, 101, 62, 107, 187, 45, 23, 58,
    193, 80, 103, 86, 209, 91, 67, 160, 178, 60, 11, 191, 161, 135, 33, 143,
    238, 139, 80, 119, 97, 41, 217, 201, 170, 45, 211, 97, 156, 165, 230, 138,
    112, 147, 73, 204, 129, 97, 184, 18, 210, 81, 131, 66, 4, 71, 74, 146,
  ]);
  const oracle = anchor.web3.Keypair.fromSecretKey(oracleSecretKey);

  let root = new Uint8Array(new Array(32).fill(0)) as any;
  const blockNumber = new anchor.BN(10);

  const [rootPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("output_root")],
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
