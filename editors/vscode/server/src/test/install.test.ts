/* The compiler a project provides (install.ts) and the ladder that consumes
 * it (ttc.findCompiler). Everything runs against fake installs in temp
 * directories — a real @load28/tt-lang is never needed, and what is
 * exercised is the published package's own `binaryPath()` contract. */
import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { test } from "node:test";

import { packageCompiler } from "../install";
import { findCompiler } from "../ttc";

const EXE = process.platform === "win32" ? "ttc.exe" : "ttc";

function scratch(prefix: string): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), prefix));
}

/**
 * A workspace with `@load28/tt-lang` installed the way npm installs it: a
 * launcher package that resolves the binary out of the per-platform package
 * its optional dependencies brought in. The files are the published
 * package's, reduced to the resolution contract this module consumes.
 *
 * @param platform the platform package's suffix — the host's, unless the
 *   test is about an install made elsewhere.
 * @param withBinary false leaves the platform package uninstalled, which is
 *   what `--omit=optional` or a foreign lockfile produces.
 */
function install(
  workspace: string,
  { platform = "host", withBinary = true } = {},
): string {
  const scope = path.join(workspace, "node_modules", "@load28");
  const platformPackage = `@load28/tt-lang-${platform}`;
  const binary = path.join(scope, `tt-lang-${platform}`, "bin", EXE);
  if (withBinary) {
    fs.mkdirSync(path.dirname(binary), { recursive: true });
    fs.writeFileSync(binary, "");
    fs.writeFileSync(
      path.join(scope, `tt-lang-${platform}`, "package.json"),
      JSON.stringify({ name: platformPackage, version: "0.0.0" }),
    );
  }

  const launcher = path.join(scope, "tt-lang");
  fs.mkdirSync(launcher, { recursive: true });
  fs.writeFileSync(
    path.join(launcher, "package.json"),
    JSON.stringify({
      name: "@load28/tt-lang",
      version: "0.0.0",
      main: "index.js",
    }),
  );
  fs.writeFileSync(
    path.join(launcher, "index.js"),
    `"use strict";
const path = require("node:path");
function binaryPath() {
  if (process.env.TTC_BINARY) return path.resolve(process.env.TTC_BINARY);
  return require.resolve(${JSON.stringify(`${platformPackage}/bin/${EXE}`)});
}
module.exports = { binaryPath };
`,
  );
  return binary;
}

test("a project's installed package provides the compiler", () => {
  const workspace = scratch("tt-install-");
  const binary = install(workspace);
  assert.equal(packageCompiler([workspace]), binary);
  // And that is what the ladder answers, with no configuration at all —
  // the case a developer who only installed the packages lands in.
  assert.equal(findCompiler("", [workspace]), binary);
});

test("the package is found from a workspace nested under the install", () => {
  const root = scratch("tt-install-mono-");
  const binary = install(root);
  const nested = path.join(root, "packages", "app");
  fs.mkdirSync(nested, { recursive: true });
  assert.equal(packageCompiler([nested]), binary);
});

test("an install missing its platform package provides nothing", () => {
  const workspace = scratch("tt-install-optional-");
  install(workspace, { withBinary: false });
  assert.equal(packageCompiler([workspace]), "");
  // The ladder falls through to PATH rather than to a path that is not there.
  assert.equal(findCompiler("", [workspace]), "ttc");
});

test("a stale install (binary gone) provides nothing", () => {
  const workspace = scratch("tt-install-stale-");
  const binary = install(workspace);
  fs.rmSync(binary);
  assert.equal(packageCompiler([workspace]), "");
});

test("a project without the package provides nothing", () => {
  const workspace = scratch("tt-install-none-");
  assert.equal(packageCompiler([workspace]), "");
});

test("a compiler built in the workspace still wins over the install", () => {
  const workspace = scratch("tt-install-repo-");
  install(workspace);
  const built = path.join(workspace, "target", "release", EXE);
  fs.mkdirSync(path.dirname(built), { recursive: true });
  fs.writeFileSync(built, "");
  assert.equal(findCompiler("", [workspace]), built);
});

test("the configured path wins over everything", () => {
  const workspace = scratch("tt-install-configured-");
  install(workspace);
  assert.equal(findCompiler("  /opt/ttc  ", [workspace]), "/opt/ttc");
});

test("a reinstall is answered by the new install, not a cached one", () => {
  const workspace = scratch("tt-install-reinstall-");
  const first = install(workspace);
  assert.equal(packageCompiler([workspace]), first);

  fs.rmSync(path.join(workspace, "node_modules"), { recursive: true });
  const second = install(workspace, { platform: "rebuilt" });
  assert.equal(packageCompiler([workspace]), second);
});
