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
  ...Buffer.from("0xf84212833806ba37257781117c119108F2145009".slice(2), "hex"),
];
export const otherBridgeAddress = [
  ...Buffer.from("0xb8947d2725D3E9De9b19fC720f053300c50981e5".slice(2), "hex"),
];
export const toAddress = Array.from({ length: 20 }, (_, i) => i);
export const dummyData = Buffer.from("sample data payload", "utf-8");
export const minGasLimit = 100000;
