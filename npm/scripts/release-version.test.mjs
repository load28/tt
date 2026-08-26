import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { setReleaseVersion } from "./release-version.mjs";

test("stamps the release branch version into Cargo and published package manifests", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "tt-release-version-"));
  fs.mkdirSync(path.join(root, "npm", "tt-lang"), { recursive: true });
  fs.mkdirSync(path.join(root, "packages", "create-tt"), { recursive: true });
  fs.writeFileSync(path.join(root, "Cargo.toml"), '[package]\nname = "ttc"\nversion = "0.3.0-dev.7"\n');
  fs.writeFileSync(path.join(root, "Cargo.lock"), '[[package]]\nname = "ttc"\nversion = "0.3.0-dev.7"\n');
  fs.writeFileSync(path.join(root, "npm", "tt-lang", "package.json"), JSON.stringify({ version: "old", optionalDependencies: { platform: "old" } }));
  fs.writeFileSync(path.join(root, "packages", "create-tt", "package.json"), JSON.stringify({ version: "old" }));

  setReleaseVersion("0.3.0-dev.8", root);

  assert.match(fs.readFileSync(path.join(root, "Cargo.toml"), "utf8"), /version = "0\.3\.0-dev\.8"/);
  assert.match(fs.readFileSync(path.join(root, "Cargo.lock"), "utf8"), /version = "0\.3\.0-dev\.8"/);
  assert.deepEqual(JSON.parse(fs.readFileSync(path.join(root, "npm", "tt-lang", "package.json"))), {
    version: "0.3.0-dev.8",
    optionalDependencies: { platform: "0.3.0-dev.8" },
  });
  assert.equal(JSON.parse(fs.readFileSync(path.join(root, "packages", "create-tt", "package.json"))).version, "0.3.0-dev.8");
});

test("accepts RC versions", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "tt-release-stage-"));
  fs.mkdirSync(path.join(root, "npm", "tt-lang"), { recursive: true });
  fs.mkdirSync(path.join(root, "packages", "create-tt"), { recursive: true });
  fs.writeFileSync(path.join(root, "Cargo.toml"), '[package]\nname = "ttc"\nversion = "0.3.0-dev.7"\n');
  fs.writeFileSync(path.join(root, "Cargo.lock"), '[[package]]\nname = "ttc"\nversion = "0.3.0-dev.7"\n');
  fs.writeFileSync(path.join(root, "npm", "tt-lang", "package.json"), JSON.stringify({ version: "old", optionalDependencies: {} }));
  fs.writeFileSync(path.join(root, "packages", "create-tt", "package.json"), JSON.stringify({ version: "old" }));

  setReleaseVersion("0.3.0-rc", root);
  assert.match(fs.readFileSync(path.join(root, "Cargo.toml"), "utf8"), /version = "0\.3\.0-rc"/);
});
