import { programConstant } from "./constants";

export function calculateGasLimit(p: {
  minGasLimit: number;
  data: Buffer;
}): number {
  const { minGasLimit, data } = p;
  const RELAY_CONSTANT_OVERHEAD = programConstant("relayConstantOverhead");
  const RELAY_CALL_OVERHEAD = programConstant("relayCallOverhead");
  const RELAY_RESERVED_GAS = programConstant("relayReservedGas");
  const RELAY_GAS_CHECK_BUFFER = programConstant("relayGasCheckBuffer");
  const MIN_GAS_DYNAMIC_OVERHEAD_NUMERATOR = programConstant(
    "minGasDynamicOverheadNumerator"
  );
  const MIN_GAS_DYNAMIC_OVERHEAD_DENOMINATOR = programConstant(
    "minGasDynamicOverheadDenominator"
  );
  const ENCODING_OVERHEAD = programConstant("encodingOverhead");
  const TX_BASE_GAS = programConstant("txBaseGas");
  const MIN_GAS_CALLDATA_OVERHEAD = programConstant("minGasCalldataOverhead");
  const FLOOR_CALLDATA_OVERHEAD = programConstant("floorCalldataOverhead");

  const execution_gas =
    RELAY_CONSTANT_OVERHEAD +
    RELAY_CALL_OVERHEAD +
    RELAY_RESERVED_GAS +
    RELAY_GAS_CHECK_BUFFER +
    (minGasLimit * MIN_GAS_DYNAMIC_OVERHEAD_NUMERATOR) /
      MIN_GAS_DYNAMIC_OVERHEAD_DENOMINATOR;

  const total_message_size = data.length + ENCODING_OVERHEAD;

  return (
    TX_BASE_GAS +
    Math.max(
      execution_gas + total_message_size * MIN_GAS_CALLDATA_OVERHEAD,
      total_message_size * FLOOR_CALLDATA_OVERHEAD
    )
  );
}
