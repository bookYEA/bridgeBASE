import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Bridge } from "../../target/types/bridge";
import { expect } from "chai";
import { solRemoteAddress } from "../utils/constants";
import { confirmTransaction } from "../utils/confirmTransaction";
import { getAccount, getMint, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { PublicKey } from "@solana/web3.js";
import { printLogs } from "../utils/printLogs";

describe("factory", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Bridge as Program<Bridge>;
  const decimals = 10;

  const [mintPda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("mint"),
      Buffer.from(solRemoteAddress),
      new anchor.BN(decimals).toBuffer("le", 1),
    ],
    program.programId
  );

  it("Should create token account", async () => {
    const tx = await program.methods
      .createMint(solRemoteAddress, decimals)
      .accounts({ tokenProgram: TOKEN_PROGRAM_ID })
      .rpc();

    await confirmTransaction(provider.connection, tx);

    const mintAccount = await getMint(provider.connection, mintPda);

    expect(mintAccount.address).to.eql(mintPda);
    expect(mintAccount.mintAuthority).to.eql(program.programId);
    expect(mintAccount.supply).to.eql(BigInt(0));
    expect(mintAccount.decimals).to.equal(decimals);
    expect(mintAccount.isInitialized).to.be.true;
    expect(mintAccount.freezeAuthority).to.eql(program.programId);
  });
});
