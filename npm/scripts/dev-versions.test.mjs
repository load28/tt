import assert from "node:assert/strict";
import test from "node:test";

import { devVersions } from "./dev-versions.mjs";

test("derives npm prereleases and a numeric VSIX version", () => {
  assert.deepEqual(
    devVersions({
      compilerBase: "0.3.0-dev.2",
      extensionBase: "0.1.0",
      unpluginBase: "0.1.0",
      timestamp: "20260823143015",
      runNumber: "412",
      runAttempt: "2",
    }),
    {
      timestamp: "20260823143015",
      npmVersion: "0.3.0-dev.2.20260823.143015.412.2",
      unpluginVersion: "0.1.0-dev.2.20260823.143015.412.2",
      vscodeVersion: "0.260823.143015",
    },
  );
});

test("rejects impossible timestamps and prerelease bases", () => {
  assert.throws(
    () =>
      devVersions({
        compilerBase: "0.3.0-dev",
        extensionBase: "0.1.0",
        unpluginBase: "0.1.0",
        timestamp: "20260230000000",
        runNumber: "1",
        runAttempt: "1",
      }),
    /compiler version/,
  );
  assert.throws(
    () =>
      devVersions({
        compilerBase: "0.3.0-dev.1",
        extensionBase: "0.1.0",
        unpluginBase: "0.1.0",
        timestamp: "20260230000000",
        runNumber: "1",
        runAttempt: "1",
      }),
    /invalid UTC timestamp/,
  );
});
