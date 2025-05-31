import { PublicKey } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";

import { Bridge } from "../../target/types/bridge";

import { deriveRoot } from "./deriveRoot";
import { confirmTransaction } from "./confirmTransaction";
import { programConstant, ORACLE_SECRET_KEY } from "./constants";
import { toNumberArray } from "./toNumberArray";

const transaction2 = toNumberArray(
  "0xb1f1d4e70a6c00ffb57d19be8bfe2dccc3695117af82b5a9183190b950fdd941"
);
const transaction3 = toNumberArray(
  "0x513a04213b8de7fc313715c0bc14e6e2e9ab7bce369818597faf2612458d93ca"
);
const transaction4 = toNumberArray(
  "0x8898b39e1f8771a1c07b2da4a191fabfc54de53b74c0fa1e82eea6de000bc424"
);

export type SetupRootAndProofResult = Awaited<
  ReturnType<typeof setupRootAndProof>
>;

export async function setupRootAndProof(p: {
  program: anchor.Program<Bridge>;
  blockNumber: anchor.BN;
  transactionHash: number[];
}): Promise<{
  rootPda: PublicKey;
  proof: number[][];
  messagePda: PublicKey;
  leafIndexBN: anchor.BN;
  totalLeafCountBN: anchor.BN;
  transaction2: number[];
}> {
  const { program, blockNumber, transactionHash } = p;
  const oracle = anchor.web3.Keypair.fromSecretKey(ORACLE_SECRET_KEY);
  const OUTPUT_ROOT_SEED = programConstant("outputRootSeed");
  const MESSAGE_SEED = programConstant("messageSeed");

  const [rootPda] = PublicKey.findProgramAddressSync(
    [Buffer.from(OUTPUT_ROOT_SEED), blockNumber.toBuffer("le", 8)],
    program.programId
  );

  const transactionsBatch = [
    transactionHash,
    transaction2,
    transaction3,
    transaction4,
  ];

  // Set MMR proof arguments based on the batch
  const leafIndexBN = new anchor.BN(0); // transactionHash is the first leaf
  const totalLeafCountBN = new anchor.BN(transactionsBatch.length);

  const { root, proof } = await deriveRoot({ batch: transactionsBatch });

  const tx = await program.methods
    .submitRoot(root as unknown as number[], blockNumber)
    .accounts({ payer: oracle.publicKey })
    .signers([oracle])
    .rpc();

  await confirmTransaction({ connection: program.provider.connection, tx });

  const [messagePda] = PublicKey.findProgramAddressSync(
    [Buffer.from(MESSAGE_SEED), Buffer.from(transactionHash)],
    program.programId
  );

  return {
    rootPda,
    proof,
    messagePda,
    leafIndexBN,
    totalLeafCountBN,
    transaction2,
  };
}
