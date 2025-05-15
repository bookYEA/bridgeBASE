export function toNumberArray(input: string): number[] {
  if (input.startsWith("0x")) {
    input = input.slice(2);
  }
  return [...Buffer.from(input, "hex")];
}
