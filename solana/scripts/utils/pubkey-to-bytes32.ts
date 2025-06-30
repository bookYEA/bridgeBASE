import { PublicKey } from "@solana/web3.js";

const PUBKEY = "EhzEY7gwHgoDzP53qUVWkdQ5SJG48hFRD9b5ba8nLdWX";

function main() {
  const pubKey = new PublicKey(PUBKEY);
  const bytes32 = pubKey.toBuffer().toString("hex");
  console.log({ bytes32 });
}

main();
