import { PublicKey } from "@solana/web3.js";

const PUBKEY = "b9GQx9Fiyg99CCyQewSAPLHhAPNnyuYBEvukkyUgAbF";

function main() {
  const pubKey = new PublicKey(PUBKEY);
  const bytes32 = pubKey.toBuffer().toString("hex");
  console.log({ bytes32 });
}

main();
