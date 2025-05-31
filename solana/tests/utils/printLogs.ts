import * as anchor from "@coral-xyz/anchor";

export async function printLogs(p: {
  connection: anchor.web3.Connection;
  tx: string;
}) {
  const { connection, tx } = p;
  const txDetails = await connection.getTransaction(tx, {
    maxSupportedTransactionVersion: 0,
    commitment: "confirmed",
  });
  const logs = txDetails?.meta?.logMessages || null;
  console.log(logs);
}
