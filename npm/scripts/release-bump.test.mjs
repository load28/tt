import assert from "node:assert/strict";
import test from "node:test";

import { releaseBumpVersion } from "./release-bump.mjs";

test("follows TypeScript's Beta, RC, stable, and patch sequence", () => {
  assert.equal(releaseBumpVersion("0.3", "0.3.0-beta"), "0.3.1-rc");
  assert.equal(releaseBumpVersion("0.3", "0.3.1-rc"), "0.3.2");
  assert.equal(releaseBumpVersion("0.3", "0.3.2"), "0.3.3");
  assert.equal(releaseBumpVersion("0.3", "0.3.8"), "0.3.9");
  assert.equal(releaseBumpVersion("0.3", "0.3.0"), "0.3.1");
});

test("rejects skipped, repeated, and cross-line transitions", () => {
  assert.throws(() => releaseBumpVersion("0.3", "0.3.1-beta"), /cannot bump/);
  assert.throws(() => releaseBumpVersion("0.3", "0.4.0-beta"), /must belong/);
  assert.throws(() => releaseBumpVersion("0.3", ""), /must belong/);
  assert.throws(() => releaseBumpVersion("03.3", "3.3.0-beta"), /must be X.Y/);
});
