import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Bridge } from "../../target/types/bridge";
import { expect } from "chai";
import { confirmTransaction } from "../utils/confirmTransaction";
import { PublicKey } from "@solana/web3.js";
import { fundAccount } from "../utils/fundAccount";
import { oracleSecretKey } from "../utils/constants";
import { toNumberArray } from "../utils/toNumberArray";
import { deriveRoot } from "../utils/deriveRoot";

describe("receiver", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;
  const payer = provider.wallet;
  const oracle = anchor.web3.Keypair.fromSecretKey(oracleSecretKey);

  const blockNumber = new anchor.BN(20);

  const [rootPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("output_root"), blockNumber.toBuffer("le", 8)],
    program.programId
  );
  let messagePda: PublicKey;
  let transactionHash: number[];
  let proof: number[][];

  before(async () => {
    await fundAccount(provider, provider.wallet.publicKey, oracle.publicKey);

    transactionHash = toNumberArray(
      "0x4492baa1c583da1575ee46a92925709e75e62fd3059e666c6982a731e50ca7b1"
    );
    const transaction2 = toNumberArray(
      "0xb1f1d4e70a6c00ffb57d19be8bfe2dccc3695117af82b5a9183190b950fdd941"
    );
    const transaction3 = toNumberArray(
      "0x513a04213b8de7fc313715c0bc14e6e2e9ab7bce369818597faf2612458d93ca"
    );
    const transaction4 = toNumberArray(
      "0x8898b39e1f8771a1c07b2da4a191fabfc54de53b74c0fa1e82eea6de000bc424"
    );

    const transactionsBatch = [
      transactionHash,
      transaction2,
      transaction3,
      transaction4,
    ];

    const { root, proof: newProof } = await deriveRoot(transactionsBatch);
    proof = newProof;

    const tx = await program.methods
      .submitRoot(root as unknown as number[], blockNumber)
      .accounts({ payer: oracle.publicKey })
      .signers([oracle])
      .rpc();

    await confirmTransaction(provider.connection, tx);

    [messagePda] = PublicKey.findProgramAddressSync(
      [Buffer.from("message"), Buffer.from(transactionHash)],
      program.programId
    );
  });

  it("Posts output root", async () => {
    const tx = await program.methods
      .proveTransaction(transactionHash, proof)
      .accounts({ payer: payer.publicKey, root: rootPda })
      .rpc();

    await confirmTransaction(provider.connection, tx);

    const message = await program.account.message.fetch(messagePda);

    expect(message.isValid).to.be.true;
  });
});
