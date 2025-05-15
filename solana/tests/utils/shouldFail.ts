import * as anchor from "@coral-xyz/anchor";
import { assert, expect } from "chai";

export async function shouldFail(fn: Promise<unknown>, expectedError?: string) {
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
