import { PublicKey } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";

import type { Bridge } from "../../target/types/bridge";

type BridgeConstants = Bridge["constants"];
type BridgeConstantNames = BridgeConstants[number]["name"];

type BridgeConstant<
  T extends BridgeConstants,
  Name extends BridgeConstantNames,
> = Extract<T[number], { name: Name }>;

type BridgeConstantField<
  T extends BridgeConstants,
  Name extends BridgeConstantNames,
  Field extends keyof BridgeConstant<T, Name> = "value",
> = BridgeConstant<T, Name>[Field];

type ParsedConstantValue<Name extends BridgeConstantNames> =
  BridgeConstantField<BridgeConstants, Name, "type"> extends "pubkey"
    ? PublicKey
    : BridgeConstantField<BridgeConstants, Name, "type"> extends
          | "u64"
          | "u16"
          | "u8"
      ? number
      : BridgeConstantField<BridgeConstants, Name, "type"> extends "bytes"
        ? number[]
        : BridgeConstantField<BridgeConstants, Name, "type"> extends {
              array: any;
            }
          ? number[]
          : string;

export const programConstant = <T extends BridgeConstantNames>(
  name: T
): ParsedConstantValue<T> => {
  const program = anchor.workspace.Bridge as Program<Bridge>;

  const constant = program.idl.constants.find((c) => c.name === name);
  if (!constant) {
    throw new Error(`Constant "${name}" not found`);
  }
  const { type, value } = constant;

  // Handle array types like { array: ["u8", 20] }
  if (typeof type === "object" && "array" in type) {
    // Value is already an array of numbers
    return JSON.parse(value) as unknown as ParsedConstantValue<T>;
  }

  // Handle primitive types
  switch (type) {
    case "pubkey":
      return new PublicKey(value) as unknown as ParsedConstantValue<T>;

    case "u64":
    case "u16":
    case "u8":
      return parseInt(value, 10) as unknown as ParsedConstantValue<T>;

    case "bytes":
      // Value is already an array of numbers
      return JSON.parse(value) as unknown as ParsedConstantValue<T>;

    default:
      // For unknown types, return the raw string value
      return value as unknown as ParsedConstantValue<T>;
  }
};

export const DUMMY_DATA = Buffer.from("sample data payload", "utf-8");

export const MIN_GAS_LIMIT = 100000;

export const ORACLE_SECRET_KEY = Uint8Array.from([
  232, 74, 68, 137, 42, 170, 245, 110, 221, 101, 62, 107, 187, 45, 23, 58, 193,
  80, 103, 86, 209, 91, 67, 160, 178, 60, 11, 191, 161, 135, 33, 143, 238, 139,
  80, 119, 97, 41, 217, 201, 170, 45, 211, 97, 156, 165, 230, 138, 112, 147, 73,
  204, 129, 97, 184, 18, 210, 81, 131, 66, 4, 71, 74, 146,
]);

export const REMOTE_TOKEN_ADDRESS = Array.from(
  Uint8Array.from(
    Buffer.from("E398D7afe84A6339783718935087a4AcE6F6DFE8", "hex")
  )
); // random address for testing
