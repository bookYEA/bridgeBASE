import { PublicKey } from "@solana/web3.js";

export class BufferReader {
  private buffer: Buffer;
  private offset: number = 0;

  constructor(buffer: Buffer) {
    this.buffer = buffer;
  }

  readUInt8(): number {
    const value = this.buffer.readUInt8(this.offset);
    this.offset += 1;
    return value;
  }

  readUInt32LE(): number {
    const value = this.buffer.readUInt32LE(this.offset);
    this.offset += 4;
    return value;
  }

  readBigUInt64LE(): bigint {
    const value = this.buffer.readBigUInt64LE(this.offset);
    this.offset += 8;
    return value;
  }

  readBytes(length: number): Buffer {
    const value = this.buffer.subarray(this.offset, this.offset + length);
    this.offset += length;
    return value;
  }

  readPublicKey(): PublicKey {
    return new PublicKey(this.readBytes(32));
  }

  readArray20(): Buffer {
    return this.readBytes(20);
  }

  getOffset(): number {
    return this.offset;
  }

  getRemainingLength(): number {
    return this.buffer.length - this.offset;
  }
}
