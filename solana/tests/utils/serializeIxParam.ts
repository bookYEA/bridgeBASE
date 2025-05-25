import { IxParam } from "./hashIxs";

/**
 * Helper function to serialize an instruction parameter
 */
export function serializeIxParam(ixParam: IxParam): Buffer {
  let serializedIxParam = Buffer.alloc(0);

  // Program ID
  serializedIxParam = Buffer.concat([
    serializedIxParam,
    ixParam.programId.toBuffer(),
  ]);

  // Accounts
  // Length of the accounts vector (u32 LE)
  const accountsLen = Buffer.alloc(4);
  accountsLen.writeUInt32LE(ixParam.accounts.length, 0);
  serializedIxParam = Buffer.concat([serializedIxParam, accountsLen]);

  for (const account of ixParam.accounts) {
    serializedIxParam = Buffer.concat([
      serializedIxParam,
      account.pubkey.toBuffer(),
    ]);
    serializedIxParam = Buffer.concat([
      serializedIxParam,
      Buffer.from([account.isWritable ? 1 : 0]),
    ]);
    serializedIxParam = Buffer.concat([
      serializedIxParam,
      Buffer.from([account.isSigner ? 1 : 0]),
    ]);
  }

  // Data
  // Length of the data vector (u32 LE)
  const dataLen = Buffer.alloc(4);
  dataLen.writeUInt32LE(ixParam.data.length, 0);
  serializedIxParam = Buffer.concat([serializedIxParam, dataLen]);
  serializedIxParam = Buffer.concat([serializedIxParam, ixParam.data]);

  return serializedIxParam;
}
