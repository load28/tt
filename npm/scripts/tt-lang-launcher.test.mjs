import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { chmodSync, existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

import { testDir } from "../../scripts/test-dirs.cjs";

const launcher = fileURLToPath(new URL("../tt-lang/bin/ttc.js", import.meta.url));

/** A stand-in for the native binary: records its pid, then runs until killed or exits with `status`. */
function fakeBinary(status) {
  const dir = testDir("tt-launcher-");
  const pidFile = join(dir, "child.pid");
  const binary = join(dir, "ttc");
  writeFileSync(
    binary,
    `#!${process.execPath}\n` +
      `require("node:fs").writeFileSync(${JSON.stringify(pidFile)}, String(process.pid));\n` +
      (status === undefined ? "setInterval(() => {}, 1000);\n" : `process.exit(${status});\n`),
  );
  chmodSync(binary, 0o755);
  return { binary, pidFile };
}

function run(binary) {
  const child = spawn(process.execPath, [launcher], {
    env: { ...process.env, TTC_BINARY: binary },
    stdio: "ignore",
  });
  const exited = new Promise((resolve) => child.on("exit", (code, signal) => resolve({ code, signal })));
  return { child, exited };
}

async function waitFor(ready) {
  const deadline = Date.now() + 10_000;
  while (!ready()) {
    assert.ok(Date.now() < deadline, "the launcher did not start its child");
    await new Promise((resolve) => setTimeout(resolve, 20));
  }
}

function alive(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    return error.code !== "ESRCH";
  }
}

test("the launcher exits with the binary's status", { skip: process.platform === "win32" }, async () => {
  const { binary } = fakeBinary(7);
  assert.deepEqual(await run(binary).exited, { code: 7, signal: null });
});

for (const signal of ["SIGTERM", "SIGINT", "SIGHUP"]) {
  test(
    `${signal} sent to the launcher ends the binary and the launcher with it`,
    { skip: process.platform === "win32" },
    async () => {
      const { binary, pidFile } = fakeBinary();
      const { child, exited } = run(binary);
      await waitFor(() => existsSync(pidFile) && readFileSync(pidFile, "utf8") !== "");
      const pid = Number(readFileSync(pidFile, "utf8"));
      child.kill(signal);
      assert.deepEqual(await exited, { code: null, signal });
      assert.equal(alive(pid), false, `the binary (pid ${pid}) outlived the launcher`);
    },
  );
}
