import { PublicKey } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import BN from "bn.js";

import { Bridge } from "../../target/types/bridge";

import { encodeMessengerData } from "./encodeMessengerData";
import { calculateGasLimit } from "./calculateGasLimit";
import { encodeBridgeData } from "./encodeBridgeData";
import { programConstant } from "./constants";
import { virtualPubkey } from "./virtualPubkey";

export function getOpaqueData(p: {
  gasLimit: BN;
  isCreation: boolean;
  data: Buffer;
}): Buffer {
  const { gasLimit, isCreation, data } = p;
  // abi.encodePacked(gasLimit, isCreation, data)
  return Buffer.concat([
    Buffer.from(gasLimit.toArray("be", 8)), // gas_limit (8 bytes, big-endian)
    Buffer.from([isCreation ? 1 : 0]), // is_creation (1 byte)
    data, // data payload
  ]);
}

export async function getOpaqueDataFromMessenger(p: {
  program: anchor.Program<Bridge>;
  extraData: Buffer;
  sender: PublicKey;
  toAddress: number[];
  minGasLimit: number;
}): Promise<Buffer> {
  const { program, extraData, sender, toAddress, minGasLimit } = p;

  const MESSENGER_SEED = programConstant("messengerSeed");
  const VERSION = new anchor.BN(programConstant("version"));

  const [messengerPda] = PublicKey.findProgramAddressSync(
    [Buffer.from(MESSENGER_SEED), VERSION.toBuffer("le", 1)],
    program.programId
  );

  const messenger = await program.account.messenger.fetch(messengerPda);

  const data = encodeMessengerData({
    nonce: messenger.msgNonce.toNumber(),
    message: extraData,
    sender,
    toAddress,
    minGasLimit,
  });

  const gasLimit = calculateGasLimit({ minGasLimit, data: extraData });

  return getOpaqueData({
    gasLimit: new BN(gasLimit),
    isCreation: false,
    data,
  });
}

export async function getOpaqueDataFromBridge(p: {
  program: anchor.Program<Bridge>;
  remoteToken: number[];
  localToken: PublicKey;
  toAddress: number[];
  value: anchor.BN;
  extraData: Buffer;
  sender: PublicKey;
  minGasLimit: number;
}): Promise<Buffer> {
  const {
    program,
    remoteToken,
    localToken,
    toAddress,
    value,
    extraData,
    sender,
    minGasLimit,
  } = p;

  const REMOTE_BRIDGE_ADDRESS = programConstant("remoteBridge");

  const message = encodeBridgeData({
    extraData,
    remoteToken,
    localToken,
    from: sender,
    target: toAddress,
    value,
  });

  const virtualBridgePubkey = virtualPubkey("bridge");

  return await getOpaqueDataFromMessenger({
    program,
    extraData: message,
    sender: virtualBridgePubkey,
    toAddress: REMOTE_BRIDGE_ADDRESS,
    minGasLimit,
  });
}
