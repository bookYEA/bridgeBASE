import * as anchor from "@coral-xyz/anchor";

export async function fundAccount(
  provider: anchor.AnchorProvider,
  from: anchor.web3.PublicKey,
  to: anchor.web3.PublicKey,
  amount?: number
) {
  // Transfer SOL from testAdmin to wallet
  const transferTransaction = new anchor.web3.Transaction().add(
    anchor.web3.SystemProgram.transfer({
      fromPubkey: from,
      toPubkey: to,
      lamports: amount ?? anchor.web3.LAMPORTS_PER_SOL,
    })
  );

  // Sign and send the transaction
  await provider.sendAndConfirm(transferTransaction);
}
