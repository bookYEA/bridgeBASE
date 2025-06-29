import { PublicKey } from "@solana/web3.js";

const PUBKEY = "EF3xsxZGWWJX9T7vCPb7hEgyJQKEj1mgSNLMNvF8a7cj";

function main() {
  const pubKey = new PublicKey(PUBKEY);
  const bytes32 = pubKey.toBuffer().toString("hex");
  console.log({ bytes32 });
}

main();
