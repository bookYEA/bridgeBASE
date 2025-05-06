const hexAddress = "0xedb3c5ab354fdd99a6e1a796117f6dc15eaf316c";

const solRemoteAddress = Uint8Array.from(
  Buffer.from(hexAddress.slice(2), "hex")
) as unknown as number[];

console.log({ hexAddress, solRemoteAddress });
