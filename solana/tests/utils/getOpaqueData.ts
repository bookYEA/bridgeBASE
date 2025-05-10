import * as anchor from "@coral-xyz/anchor";
import BN from "bn.js";
import { encodeMessengerData } from "./encodeMessengerData";
import { calculateGasLimit } from "./calculateGasLimit";
import { PublicKey } from "@solana/web3.js";
import { Bridge } from "../../target/types/bridge";
import { encodeBridgeData } from "./encodeBridgeData";
import { expectedBridgePubkey, otherBridgeAddress } from "./constants";

export function getOpaqueData(
  gasLimit: BN,
  isCreation: boolean,
  data: Buffer
): Buffer {
  // abi.encodePacked(gasLimit, isCreation, data)
  return Buffer.concat([
    Buffer.from(gasLimit.toArray("be", 8)), // gas_limit (8 bytes, big-endian)
    Buffer.from([isCreation ? 1 : 0]), // is_creation (1 byte)
    data, // data payload
  ]);
}

export async function getOpaqueDataFromMessenger(
  program: anchor.Program<Bridge>,
  extraData: Buffer,
  sender: PublicKey,
  toAddress: number[],
  minGasLimit: number
): Promise<Buffer> {
  const [messengerPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("messenger_state")],
    program.programId
  );

  const messenger = await program.account.messenger.fetch(messengerPda);

  const data = encodeMessengerData(
    messenger.msgNonce.toNumber(),
    extraData,
    sender,
    toAddress,
    minGasLimit
  );

  const gasLimit = calculateGasLimit(minGasLimit, extraData);

  return getOpaqueData(new BN(gasLimit), false, data);
}

export async function getOpaqueDataFromBridge(
  program: anchor.Program<Bridge>,
  remoteToken: number[],
  localToken: PublicKey,
  toAddress: number[],
  value: anchor.BN,
  extraData: Buffer,
  sender: PublicKey,
  minGasLimit: number
): Promise<Buffer> {
  const message = encodeBridgeData(
    extraData,
    remoteToken,
    localToken,
    sender,
    toAddress,
    value
  );

  return await getOpaqueDataFromMessenger(
    program,
    message,
    expectedBridgePubkey,
    otherBridgeAddress,
    minGasLimit
  );
}
