import * as anchor from "@coral-xyz/anchor";

export async function confirmTransaction(
  connection: anchor.web3.Connection,
  tx: string
) {
  const latestBlockHash = await connection.getLatestBlockhash();
  await connection.confirmTransaction(
    {
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: tx,
    },
    "confirmed"
  );
}
