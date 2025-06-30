import { PublicKey } from "@solana/web3.js";

const PUBKEY = "6bmM7CK2yfP4M7KGHmb6Q3b7yCKcdkGQYKszSLwwfpmD";

function main() {
  const pubKey = new PublicKey(PUBKEY);
  const bytes32 = pubKey.toBuffer().toString("hex");
  console.log({ bytes32 });
}

main();
