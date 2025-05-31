import * as anchor from "@coral-xyz/anchor";

export async function confirmTransaction(p: {
  connection: anchor.web3.Connection;
  tx: string;
}) {
  const { connection, tx } = p;
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

// Utility function to handle event listening and transaction execution
export async function executeWithEventListener<T extends anchor.Idl>(p: {
  program: anchor.Program<T>;
  provider: anchor.AnchorProvider;
  transactionFn: () => Promise<string>;
}): Promise<{ event: any; slot: number }> {
  const { program, provider, transactionFn } = p;
  return new Promise(async (resolve, reject) => {
    let listener = null;

    listener = program.addEventListener(
      "transactionDeposited",
      async (event, slot) => {
        await program.removeEventListener(listener);
        resolve({ event, slot });
      }
    );

    try {
      const tx = await transactionFn();
      await confirmTransaction({ connection: provider.connection, tx });
    } catch (e) {
      await program.removeEventListener(listener);
      reject(e);
    }
  });
}
