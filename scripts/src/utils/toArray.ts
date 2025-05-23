export function toArray(a: string): number[] {
  if (a.startsWith("0x")) {
    a = a.slice(2);
  }
  return Array.from(Buffer.from(a, "hex"));
}
