import assert from "node:assert/strict";
import test from "node:test";

import { releaseStageVersion } from "./release-stage.mjs";

test("advances through the RC, stable, and patch sequence", () => {
  assert.equal(releaseStageVersion("rc", "0.3"), "0.3.0-rc");
  assert.equal(releaseStageVersion("stable", "0.3", "0.3.0-rc"), "0.3.0");
  assert.equal(releaseStageVersion("patch", "0.3", "0.3.0"), "0.3.1");
});

test("rejects skipped stages and versions from another release line", () => {
  assert.throws(() => releaseStageVersion("stable", "0.3", "0.3.0"), /cannot advance/);
  assert.throws(() => releaseStageVersion("stable", "0.3", "0.4.0-rc"), /must belong/);
  assert.throws(() => releaseStageVersion("rc", "0.3", "0.3.0-rc"), /already exists/);
});
