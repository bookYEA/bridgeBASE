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
import { hashIxs, IxParam } from "../utils/hashIxs";
import { shouldFail } from "../utils/shouldFail";

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
  let proof: number[][];

  const TRANSFER_AMOUNT = 1 * anchor.web3.LAMPORTS_PER_SOL;

  const transferIx = anchor.web3.SystemProgram.transfer({
    fromPubkey: payer.publicKey,
    toPubkey: oracle.publicKey,
    lamports: TRANSFER_AMOUNT,
  });

  const transferIxParam: IxParam = {
    programId: transferIx.programId,
    accounts: transferIx.keys,
    data: transferIx.data,
  };
  const transactionHash = toNumberArray(hashIxs([transferIxParam]));
  let transaction2: number[];

  before(async () => {
    await fundAccount(provider, provider.wallet.publicKey, oracle.publicKey);

    transaction2 = toNumberArray(
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

  describe("Prove transaction", () => {
    it("Should fail if invalid transaction hash", async () => {
      await shouldFail(
        program.methods
          .proveTransaction(transaction2, [transferIxParam], proof)
          .accounts({ payer: payer.publicKey, root: rootPda })
          .rpc(),
        "Invalid transaction hash"
      );
    });

    it("Should fail if invalid proof", async () => {
      const badProof = structuredClone(proof);
      badProof.pop();

      await shouldFail(
        program.methods
          .proveTransaction(transactionHash, [transferIxParam], badProof)
          .accounts({ payer: payer.publicKey, root: rootPda })
          .rpc(),
        "Invalid proof"
      );
    });

    it("Posts output root", async () => {
      const tx = await program.methods
        .proveTransaction(transactionHash, [transferIxParam], proof)
        .accounts({ payer: payer.publicKey, root: rootPda })
        .rpc();

      await confirmTransaction(provider.connection, tx);

      const message = await program.account.message.fetch(messagePda);

      expect(message.ixs).to.eql([transferIxParam]);
      expect(message.isExecuted).to.be.false;
    });
  });

  describe("Finalize transaction", () => {
    before(async () => {
      const tx = await program.methods
        .finalizeTransaction(transactionHash)
        .accounts({})
        .remainingAccounts([
          ...transferIxParam.accounts,
          {
            pubkey: anchor.web3.SystemProgram.programId,
            isSigner: false,
            isWritable: false,
          },
        ])
        .rpc();

      await confirmTransaction(provider.connection, tx);
    });

    it("Should execute transaction", async () => {
      const message = await program.account.message.fetch(messagePda);
      expect(message.isExecuted).to.be.true;
    });

    it("Should fail if already executed", async () => {
      await shouldFail(
        program.methods
          .finalizeTransaction(transactionHash)
          .accounts({})
          .remainingAccounts([
            ...transferIxParam.accounts,
            {
              pubkey: anchor.web3.SystemProgram.programId,
              isSigner: false,
              isWritable: false,
            },
          ])
          .rpc(),
        "Already executed"
      );
    });
  });
});
