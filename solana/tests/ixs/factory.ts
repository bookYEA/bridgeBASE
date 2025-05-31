import { expect } from "chai";
import { PublicKey } from "@solana/web3.js";
import { getMint, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";

import { Bridge } from "../../target/types/bridge";

import { programConstant, REMOTE_TOKEN_ADDRESS } from "../utils/constants";
import { confirmTransaction } from "../utils/confirmTransaction";

describe("factory", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Bridge as Program<Bridge>;

  // Test constants
  const DECIMALS = 9;

  // Program constants
  const MINT_SEED = programConstant("mintSeed");

  // PDAs
  const [mintPda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from(MINT_SEED),
      Buffer.from(REMOTE_TOKEN_ADDRESS),
      new anchor.BN(DECIMALS).toBuffer("le", 1),
    ],
    program.programId
  );

  it("Creates a mint account", async () => {
    const tx = await program.methods
      .createMint(REMOTE_TOKEN_ADDRESS, DECIMALS)
      .accounts({ tokenProgram: TOKEN_PROGRAM_ID })
      .rpc();

    await confirmTransaction({ connection: provider.connection, tx });

    const mintAccount = await getMint(provider.connection, mintPda);

    expect(mintAccount.address).to.eql(mintPda);
    expect(mintAccount.mintAuthority).to.eql(mintPda);
    expect(mintAccount.supply).to.eql(BigInt(0));
    expect(mintAccount.decimals).to.equal(DECIMALS);
    expect(mintAccount.isInitialized).to.be.true;
    expect(mintAccount.freezeAuthority).to.eql(mintPda);
  });
});
