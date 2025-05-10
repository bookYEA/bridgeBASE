export function calculateGasLimit(minGasLimit: number, data: Buffer): number {
  const execution_gas =
    200_000 + 40_000 + 40_000 + 5_000 + (minGasLimit * 64) / 63;
  const total_message_size = data.length + 260;

  return (
    21_000 +
    Math.max(execution_gas + total_message_size * 16, total_message_size * 40)
  );
}
