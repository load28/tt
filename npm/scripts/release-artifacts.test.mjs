import assert from "node:assert/strict";
import test from "node:test";

import { nightlyVersion, releaseArtifacts } from "./release-artifacts.mjs";

test("derives TypeScript-style dated nightlies from main", () => {
  assert.equal(nightlyVersion("0.4.0-dev.1", "20260826123456", "42"), "0.4.0-dev.20260826.42");
  assert.equal(nightlyVersion("0.4.0-dev.1", "20260826123456", "43"), "0.4.0-dev.20260826.43");
  assert.deepEqual(releaseArtifacts({
    compilerVersion: "0.4.0-dev.20260826.42", extensionBase: "0.1.0", unpluginBase: "0.1.0", timestamp: "20260826123456",
  }), {
    version: "0.4.0-dev.20260826.42", npmTag: "next", unpluginVersion: "0.1.0-next.0.4.0.20260826.42", vscodeVersion: "0.260826.123456",
  });
});

test("maps release stages to npm tags", () => {
  const common = { extensionBase: "0.1.0", unpluginBase: "0.1.0", timestamp: "20260826123456" };
  assert.equal(releaseArtifacts({ ...common, compilerVersion: "0.3.0-beta" }).npmTag, "beta");
  assert.equal(releaseArtifacts({ ...common, compilerVersion: "0.3.1-rc" }).npmTag, "rc");
  assert.equal(releaseArtifacts({ ...common, compilerVersion: "0.3.2" }).npmTag, "latest");
});

test("removes leading zeroes from the VS Code numeric version component", () => {
  assert.equal(releaseArtifacts({
    compilerVersion: "0.4.0-dev.20260826.42", extensionBase: "0.1.0", unpluginBase: "0.1.0", timestamp: "20260826001724",
  }).vscodeVersion, "0.260826.1724");
});
