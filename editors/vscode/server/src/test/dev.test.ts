/* The local-development toolchain resolution (dev.ts): what `scripts/setup`
 * writes is what the server hands to every ttc it spawns — and nothing is
 * handed over when the setup is absent, chose npm, or went stale. All on
 * temp directories; no real toolchain is involved. */
import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { test } from "node:test";

import { devPackageCompiler, ttcSpawnEnv, toolchainEnv } from "../dev";

/** A fake TT repository whose setup chose the given toolchain config. */
function ttRepo(toolchain: object | null): string {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "tt-dev-test-"));
  fs.mkdirSync(path.join(root, "target", "release"), { recursive: true });
  if (toolchain !== null) {
    fs.mkdirSync(path.join(root, ".tt-dev"));
    fs.writeFileSync(
      path.join(root, ".tt-dev", "toolchain.json"),
      JSON.stringify(toolchain),
    );
  }
  return root;
}

/** A fake built typescript-go checkout (both artifacts present). */
function builtTsgo(): string {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "tt-tsgo-test-"));
  fs.mkdirSync(path.join(root, "built", "local"), { recursive: true });
  fs.writeFileSync(path.join(root, "built", "local", "tsgo"), "");
  const api = path.join(root, "_packages", "native-preview", "dist", "api", "sync");
  fs.mkdirSync(api, { recursive: true });
  fs.writeFileSync(path.join(api, "api.js"), "");
  return root;
}

test("a checkout toolchain above the compiler becomes TTC_TSGO_* variables", () => {
  const tsgo = builtTsgo();
  const tt = ttRepo({ kind: "checkout", root: tsgo });
  const env = toolchainEnv(path.join(tt, "target", "release", "ttc"));
  assert.deepEqual(env, {
    TTC_TSGO_ROOT: tsgo,
    TTC_TSGO_BIN: path.join(tsgo, "built", "local", "tsgo"),
    TTC_TSGO_API: path.join(
      tsgo,
      "_packages",
      "native-preview",
      "dist",
      "api",
      "sync",
      "api.js",
    ),
  });
});

test("a pre-'kind' config (root only) still reads as a checkout", () => {
  const tsgo = builtTsgo();
  const tt = ttRepo({ root: tsgo });
  const env = toolchainEnv(path.join(tt, "target", "release", "ttc"));
  assert.equal(env?.TTC_TSGO_ROOT, tsgo);
});

test("the npm toolchain adds nothing — ttc resolves the project's own", () => {
  const tt = ttRepo({ kind: "npm" });
  assert.equal(toolchainEnv(path.join(tt, "target", "release", "ttc")), null);
});

test("an unbuilt checkout adds nothing rather than a broken TTC_TSGO_ROOT", () => {
  const tsgo = fs.mkdtempSync(path.join(os.tmpdir(), "tt-tsgo-test-"));
  const tt = ttRepo({ kind: "checkout", root: tsgo });
  assert.equal(toolchainEnv(path.join(tt, "target", "release", "ttc")), null);
});

test("no config, a bare PATH compiler: inherit the environment untouched", () => {
  const tt = ttRepo(null);
  assert.equal(toolchainEnv(path.join(tt, "target", "release", "ttc")), null);
  assert.equal(toolchainEnv("ttc"), null);
  assert.equal(ttcSpawnEnv("ttc"), undefined);
});

test("ttcSpawnEnv layers the toolchain over the process environment", () => {
  const tsgo = builtTsgo();
  const tt = ttRepo({ kind: "checkout", root: tsgo });
  const env = ttcSpawnEnv(path.join(tt, "target", "release", "ttc"));
  assert.equal(env?.TTC_TSGO_ROOT, tsgo);
  assert.equal(env?.PATH, process.env.PATH);
});

test("devPackageCompiler finds the ttc of a file:-installed dev package", () => {
  const tt = ttRepo(null);
  const exe = process.platform === "win32" ? "ttc.exe" : "ttc";
  fs.writeFileSync(path.join(tt, "target", "release", exe), "");

  const workspace = fs.mkdtempSync(path.join(os.tmpdir(), "tt-ws-test-"));
  const pkg = path.join(workspace, "node_modules", "@load28", "tt-lang");
  fs.mkdirSync(pkg, { recursive: true });
  fs.writeFileSync(
    path.join(pkg, "tt-dev.local.json"),
    JSON.stringify({ root: tt }),
  );

  assert.equal(
    devPackageCompiler([workspace]),
    path.join(tt, "target", "release", exe),
  );

  // A stale stamp (binary gone) resolves to nothing, not to a dead path.
  fs.rmSync(path.join(tt, "target", "release", exe));
  assert.equal(devPackageCompiler([workspace]), "");
});
