import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createRequire } from "node:module";
import test from "node:test";

const require = createRequire(import.meta.url);
const platforms = require("../tt-lang/platforms.json");
const manifest = require("../tt-lang/package.json");
const { platformPackageName } = require("../tt-lang/index.js");

test("the main package depends on every declared platform package", () => {
  assert.deepEqual(
    Object.keys(manifest.optionalDependencies).sort(),
    Object.values(platforms)
      .map((target) => target.package)
      .sort(),
  );
});

test("the Windows runtime key resolves to the available MSVC package name", () => {
  assert.equal(platformPackageName("win32-x64"), "tt-lang-win32-x64-msvc");
  assert.equal(platformPackageName("freebsd-x64"), undefined);
});

test("the package assembler keeps the build key but stamps the mapped npm name", () => {
  const root = mkdtempSync(join(tmpdir(), "tt-platform-package-"));
  try {
    const binary = join(root, "ttc.exe");
    const output = join(root, "packages");
    writeFileSync(binary, "test binary");

    execFileSync(
      process.execPath,
      [
        new URL("./make-platform-package.mjs", import.meta.url).pathname,
        "win32-x64",
        binary,
        output,
        "0.3.0-dev.1",
      ],
      { stdio: "pipe" },
    );

    const generated = JSON.parse(
      readFileSync(join(output, "tt-lang-win32-x64-msvc", "package.json"), "utf8"),
    );
    assert.equal(generated.name, "tt-lang-win32-x64-msvc");
    assert.deepEqual(generated.os, ["win32"]);
    assert.deepEqual(generated.cpu, ["x64"]);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
