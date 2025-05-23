import { PublicKey } from "@solana/web3.js";

export const expectedMessengerPubkey = new PublicKey(
  Buffer.from(
    "7e273983f136714ba93a740a050279b541d6f25ebc6bbc6fc67616d0d5529cea",
    "hex"
  )
);
export const expectedBridgePubkey = new PublicKey(
  Buffer.from(
    "7a25452c36304317d6fe970091c383b0d45e9b0b06485d2561156f025c6936af",
    "hex"
  )
);
export const otherMessengerAddress = [
  ...Buffer.from("0x463e2daAf0bdaC35F022E5478f710257b5830DdB".slice(2), "hex"),
];
export const otherBridgeAddress = [
  ...Buffer.from("0x32148f9a788d89677a4a0518C2AcA9666A61fBBa".slice(2), "hex"),
];
export const toAddress = Array.from({ length: 20 }, (_, i) => i);
export const dummyData = Buffer.from("sample data payload", "utf-8");
export const minGasLimit = 100000;
export const oracleSecretKey = Uint8Array.from([
  232, 74, 68, 137, 42, 170, 245, 110, 221, 101, 62, 107, 187, 45, 23, 58, 193,
  80, 103, 86, 209, 91, 67, 160, 178, 60, 11, 191, 161, 135, 33, 143, 238, 139,
  80, 119, 97, 41, 217, 201, 170, 45, 211, 97, 156, 165, 230, 138, 112, 147, 73,
  204, 129, 97, 184, 18, 210, 81, 131, 66, 4, 71, 74, 146,
]);
export const solRemoteAddress = Array.from(
  Uint8Array.from(
    Buffer.from("E398D7afe84A6339783718935087a4AcE6F6DFE8", "hex")
  )
); // random address for testing
export const VERSION = 2;
