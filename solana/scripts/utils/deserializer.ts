import { PublicKey } from "@solana/web3.js";
import { BufferReader } from "./buffer-reader";

type Message = Call | Transfer;

type Call = {
  type: "Call";
  ixs: ReturnType<typeof deserializeIx>[];
};

type Transfer = {
  type: "Transfer";
  transfer:
    | {
        type: "Sol";
        remoteToken: Buffer;
        to: PublicKey;
        amount: bigint;
      }
    | {
        type: "Spl";
        remoteToken: Buffer;
        localToken: PublicKey;
        to: PublicKey;
        amount: bigint;
      }
    | {
        type: "WrappedToken";
        localToken: PublicKey;
        to: PublicKey;
        amount: bigint;
      };

  ixs: ReturnType<typeof deserializeIx>[];
};

export function deserializeMessage(buffer: Buffer): Message {
  const reader = new BufferReader(buffer);

  // Read Message enum discriminator (1 byte)
  const messageDiscriminator = reader.readUInt8();

  // Message::Call(Vec<Ix>)
  if (messageDiscriminator === 0) {
    const ixs = deserializeIxs(reader);
    return { type: "Call", ixs };
  }
  // Message::Transfer { transfer: Transfer, ixs: Vec<Ix> }
  else if (messageDiscriminator === 1) {
    // Read Transfer enum discriminator (1 byte)
    const transferDiscriminator = reader.readUInt8();

    let transfer: Transfer["transfer"];
    // Sol(FinalizeBridgeSol)
    if (transferDiscriminator === 0) {
      const remoteToken = reader.readArray20(); // [u8; 20]
      const to = reader.readPublicKey(); // Pubkey
      const amount = reader.readBigUInt64LE(); // u64

      transfer = { type: "Sol", remoteToken, to, amount };
    }
    // Spl(FinalizeBridgeSpl)
    else if (transferDiscriminator === 1) {
      const remoteToken = reader.readArray20(); // [u8; 20]
      const localToken = reader.readPublicKey(); // Pubkey (mint)
      const to = reader.readPublicKey(); // Pubkey
      const amount = reader.readBigUInt64LE(); // u64

      transfer = { type: "Spl", remoteToken, localToken, to, amount };
    }
    // WrappedToken(FinalizeBridgeWrappedToken)
    else if (transferDiscriminator === 2) {
      const localToken = reader.readPublicKey(); // Pubkey (mint)
      const to = reader.readPublicKey(); // Pubkey
      const amount = reader.readBigUInt64LE(); // u64

      transfer = { type: "WrappedToken", localToken, to, amount };
    }
    // Unknown transfer discriminator
    else {
      throw new Error(
        `Unknown transfer discriminator: ${transferDiscriminator}`
      );
    }

    // Read Vec<Ix> after the transfer
    const ixs = deserializeIxs(reader);

    return { type: "Transfer", transfer, ixs } as const;
  }

  throw new Error(`Unknown discriminator: ${messageDiscriminator}`);
}

function deserializeIxs(reader: BufferReader) {
  // Read Vec length (4 bytes)
  const ixsLength = reader.readUInt32LE();

  // Read instructions
  const ixs = [];
  for (let i = 0; i < ixsLength; i++) {
    const ix = deserializeIx(reader);
    ixs.push(ix);
  }

  return ixs;
}

function deserializeIx(reader: BufferReader) {
  // Read program_id (32 bytes)
  const programId = reader.readPublicKey();

  // Read accounts Vec length (4 bytes)
  const accountsLength = reader.readUInt32LE();

  // Read accounts
  const accounts = [];
  for (let i = 0; i < accountsLength; i++) {
    const account = deserializeIxAccount(reader);
    accounts.push(account);
  }

  // Read data Vec length (4 bytes)
  const dataLength = reader.readUInt32LE();

  // Read data
  const data = reader.readBytes(dataLength);

  return { programId, accounts, data };
}

function deserializeIxAccount(reader: BufferReader) {
  const pubkey = deserializePubkeyOrPda(reader);

  const isWritable = reader.readUInt8() === 1;
  const isSigner = reader.readUInt8() === 1;

  return { pubkey, isWritable, isSigner };
}

function deserializePubkeyOrPda(reader: BufferReader) {
  const discriminator = reader.readUInt8();

  // Pubkey variant
  if (discriminator === 0) {
    return reader.readPublicKey();
  }
  // PDA variant
  else if (discriminator === 1) {
    const seedsLength = reader.readUInt32LE();

    const seeds: Buffer[] = [];
    for (let i = 0; i < seedsLength; i++) {
      const seedLength = reader.readUInt32LE();
      const seed = reader.readBytes(seedLength);
      seeds.push(seed);
    }

    const programId = reader.readPublicKey();

    // Derive the PDA
    const [pubkey] = PublicKey.findProgramAddressSync(seeds, programId);
    return pubkey;
  }

  throw new Error(`Unknown PubkeyOrPda discriminator: ${discriminator}`);
}
