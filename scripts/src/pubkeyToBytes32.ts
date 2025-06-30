import { PublicKey } from "@solana/web3.js";

const PUBKEY = "7GcQjDnXnD4fL9N3FQAMghubZ6Jv5e32ouBbKSEjpNXx";

function main() {
  const pubKey = new PublicKey(PUBKEY);
  const bytes32 = pubKey.toBuffer().toString("hex");
  console.log({ bytes32 });
}

main();
