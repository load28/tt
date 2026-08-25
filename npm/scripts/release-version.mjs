import * as fs from "node:fs";
import * as path from "node:path";
import { pathToFileURL } from "node:url";

import { stampVersion } from "./stamp-version.mjs";

const VERSION = /^(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)(?:-dev\.[1-9]\d*)?$/;

export function setReleaseVersion(version, repositoryRoot = process.cwd()) {
  if (!VERSION.test(version)) throw new Error(`release version must be X.Y.Z or X.Y.Z-dev.N: ${version}`);
  replaceOnce(
    path.join(repositoryRoot, "Cargo.toml"),
    /(^\[package\]\nname = "ttc"\nversion = ")[^"]+("$)/m,
    `$1${version}$2`,
  );
  replaceOnce(
    path.join(repositoryRoot, "Cargo.lock"),
    /(\[\[package\]\]\nname = "ttc"\nversion = ")[^"]+("\n)/,
    `$1${version}$2`,
  );
  stampVersion(version, [], repositoryRoot);
}

function replaceOnce(file, pattern, replacement) {
  const source = fs.readFileSync(file, "utf8");
  if (!pattern.test(source)) throw new Error(`${file} ttc version was not found`);
  const updated = source.replace(pattern, replacement);
  fs.writeFileSync(file, updated);
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    if (process.argv.length !== 3) throw new Error("usage: release-version.mjs <X.Y.Z[-dev.N]>");
    setReleaseVersion(process.argv[2]);
  } catch (error) {
    console.error(`release-version: ${error.message}`);
    process.exit(1);
  }
}
