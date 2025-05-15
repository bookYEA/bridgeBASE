import * as anchor from "@coral-xyz/anchor";

export async function printLogs(
  connection: anchor.web3.Connection,
  tx: string
) {
  const txDetails = await connection.getTransaction(tx, {
    maxSupportedTransactionVersion: 0,
    commitment: "confirmed",
  });
  const logs = txDetails?.meta?.logMessages || null;
  console.log(logs);
}
