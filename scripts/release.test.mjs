import assert from "node:assert/strict";
import test from "node:test";
import { assertPatchUpgrade, assertReleaseSequence, nextPatchVersion } from "./release.mjs";

test("computes and accepts only the immediate next patch", () => {
  assert.equal(nextPatchVersion("0.1.0"), "0.1.1");
  assert.doesNotThrow(() => assertPatchUpgrade("0.1.0", "0.1.1"));
});

test("rejects minor, major, skipped, repeated, downgraded, and prerelease versions", () => {
  for (const version of ["0.2.0", "1.0.0", "0.1.2", "0.1.0", "0.0.9", "0.1.1-beta.1"]) {
    assert.throws(() => assertPatchUpgrade("0.1.0", version), /Patch-only releases require 0\.1\.1/);
  }
});

test("anchors the first release and every later tag to one patch step", () => {
  assert.doesNotThrow(() => assertReleaseSequence(undefined, "0.1.1"));
  assert.doesNotThrow(() => assertReleaseSequence("0.1.1", "0.1.2"));
  assert.throws(() => assertReleaseSequence(undefined, "0.2.0"), /require 0\.1\.1/);
  assert.throws(() => assertReleaseSequence("0.1.1", "0.1.3"), /require 0\.1\.2/);
});
