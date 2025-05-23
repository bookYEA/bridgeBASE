import { createPublicClient, formatUnits, http, type Address } from "viem";
import { baseSepolia } from "viem/chains";
import baseSepoliaAddrs from "../deployments/base_sepolia.json";
import { loadFromEnv } from "./utils/loadFromEnv";

async function main() {
  const publicClient = createPublicClient({
    chain: baseSepolia,
    transport: http(),
  });

  const res = await publicClient.readContract({
    address: baseSepoliaAddrs.WrappedSPL as Address,
    abi: [
      {
        type: "function",
        name: "balanceOf",
        inputs: [
          {
            name: "owner",
            type: "address",
            internalType: "address",
          },
        ],
        outputs: [
          {
            name: "result",
            type: "uint256",
            internalType: "uint256",
          },
        ],
        stateMutability: "view",
      },
    ],
    functionName: "balanceOf",
    args: [loadFromEnv("USER") as Address],
  });
  console.log(`Base balance: ${formatUnits(res, 9)}`);
}

main().catch(console.error);
