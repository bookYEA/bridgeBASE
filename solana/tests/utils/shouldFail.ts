import { assert, expect } from "chai";
import * as anchor from "@coral-xyz/anchor";

export async function shouldFail(p: {
  fn: Promise<unknown>;
  expectedError?: string;
}) {
  const { fn, expectedError } = p;
  try {
    await fn;
    assert(false, "should've failed but didn't");
  } catch (e) {
    if (expectedError) {
      expect(e).to.be.instanceOf(anchor.AnchorError);
      expect((e as anchor.AnchorError).error.errorMessage).to.equal(
        expectedError
      );
    }
  }
}
