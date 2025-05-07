const hexAddress = process.env.ADDRESS as string;

const solRemoteAddress = Uint8Array.from(
  Buffer.from(hexAddress.slice(2), "hex")
) as unknown as number[];

console.log({ hexAddress, solRemoteAddress });
