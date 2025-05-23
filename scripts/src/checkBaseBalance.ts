import { createPublicClient, formatEther, formatUnits, http } from "viem";
import { baseSepolia } from "viem/chains";

const wSPL = "0x7aBc6d57A03f3b3eeA91fc2151638A549050eB42";
const user = "0x8C1a617BdB47342F9C17Ac8750E0b070c372C721";

async function main() {
  const publicClient = createPublicClient({
    chain: baseSepolia,
    transport: http(),
  });

  const res = await publicClient.readContract({
    address: wSPL,
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
    args: [user],
  });
  console.log(`Base balance: ${formatUnits(res, 9)}`);
}

main().catch(console.error);
