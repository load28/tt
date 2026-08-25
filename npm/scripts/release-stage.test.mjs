import assert from "node:assert/strict";
import test from "node:test";

import { releaseStageVersion } from "./release-stage.mjs";

test("follows the TypeScript beta, RC, stable, and patch sequence", () => {
  assert.equal(releaseStageVersion("beta", "0.3"), "0.3.0-beta");
  assert.equal(releaseStageVersion("rc", "0.3", "0.3.0-beta"), "0.3.1-rc");
  assert.equal(releaseStageVersion("stable", "0.3", "0.3.1-rc"), "0.3.2");
  assert.equal(releaseStageVersion("patch", "0.3", "0.3.2"), "0.3.3");
});

test("rejects skipped stages and versions from another release line", () => {
  assert.throws(() => releaseStageVersion("stable", "0.3", "0.3.0-beta"), /cannot advance/);
  assert.throws(() => releaseStageVersion("rc", "0.3", "0.4.0-beta"), /must belong/);
  assert.throws(() => releaseStageVersion("beta", "0.3", "0.3.0-beta"), /already exists/);
});
