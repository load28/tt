import assert from "node:assert/strict";
import test from "node:test";

import { releaseChannel, requireDevelopment, requireProduction } from "./release-channel.mjs";

test("routes changed development and production versions", () => {
  assert.equal(releaseChannel("0.3.0", "0.3.0-dev.1"), "development");
  assert.equal(releaseChannel("0.3.0-dev.1", "0.3.0-dev.2"), "development");
  assert.equal(releaseChannel("0.3.0-dev.2", "0.3.1"), "production");
  assert.equal(releaseChannel("0.3.0", "0.3.1"), "production");
});

test("skips an unchanged version", () => {
  assert.equal(releaseChannel("0.3.0-dev.1", "0.3.0-dev.1"), "none");
  assert.equal(releaseChannel("0.3.0", "0.3.0"), "none");
});

test("rejects regressions and unsupported prerelease forms", () => {
  assert.throws(() => releaseChannel("0.3.0-dev.2", "0.3.0-dev.1"), /does not increase/);
  assert.throws(() => releaseChannel("0.3.1", "0.3.0-dev.3"), /does not increase/);
  assert.throws(() => releaseChannel("0.3.0", "0.3.1-beta.1"), /must be/);
});

test("manual development runs require an explicit dev version", () => {
  assert.equal(requireDevelopment("0.3.0-dev.1"), "development");
  assert.throws(() => requireDevelopment("0.3.0"), /requires X.Y.Z-dev.N/);
});

test("production releases require a stable version", () => {
  assert.equal(requireProduction("0.3.1"), "production");
  assert.throws(() => requireProduction("0.3.1-dev.1"), /requires X.Y.Z/);
});
