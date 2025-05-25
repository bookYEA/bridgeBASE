export default [
  {
    type: "event",
    name: "MessagePassed",
    inputs: [
      {
        name: "nonce",
        type: "uint256",
        indexed: true,
        internalType: "uint256",
      },
      {
        name: "sender",
        type: "address",
        indexed: true,
        internalType: "address",
      },
      {
        name: "ixs",
        type: "tuple[]",
        indexed: false,
        internalType: "struct MessagePasser.Instruction[]",
        components: [
          {
            name: "programId",
            type: "bytes32",
            internalType: "bytes32",
          },
          {
            name: "accounts",
            type: "tuple[]",
            internalType: "struct MessagePasser.AccountMeta[]",
            components: [
              {
                name: "pubKey",
                type: "bytes32",
                internalType: "bytes32",
              },
              {
                name: "isSigner",
                type: "bool",
                internalType: "bool",
              },
              {
                name: "isWritable",
                type: "bool",
                internalType: "bool",
              },
            ],
          },
          {
            name: "data",
            type: "bytes",
            internalType: "bytes",
          },
        ],
      },
      {
        name: "withdrawalHash",
        type: "bytes32",
        indexed: false,
        internalType: "bytes32",
      },
    ],
    anonymous: false,
  },
] as const;
