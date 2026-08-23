#!/usr/bin/env node
/* --------------------------------------------------------------------------
 * `ttc` launcher — thin veneer over the native binary from the platform
 * package. All arguments, stdio and the exit code pass through untouched,
 * so `npx ttc` behaves exactly like a natively installed ttc.
 *
 * In a local development install (`scripts/setup` in the TT repository,
 * then `pnpm add -D file:.../npm/tt-lang`) the binary is the repository's
 * release build instead, and the TypeScript toolchain setup configured is
 * handed to it as TTC_TSGO_* variables — set on this child process only,
 * never on the user's shell (dev.js).
 * ----------------------------------------------------------------------- */
"use strict";

const { spawnSync } = require("node:child_process");

const { binaryPath } = require("../index.js");
const { devEnvironment } = require("../dev.js");

let binary;
let env = process.env;
try {
  // TTC_BINARY stays the strongest override, exactly as in binaryPath().
  const dev = process.env.TTC_BINARY ? null : devEnvironment();
  if (dev) {
    binary = dev.binary;
    env = { ...process.env, ...dev.env };
  } else {
    binary = binaryPath();
  }
} catch (error) {
  console.error(error.message);
  process.exit(1);
}

const result = spawnSync(binary, process.argv.slice(2), {
  stdio: "inherit",
  env,
});
if (result.error) {
  console.error(`ttc: failed to run ${binary}: ${result.error.message}`);
  process.exit(1);
}
if (result.signal) {
  // Re-raise so the parent observes the same termination signal.
  process.kill(process.pid, result.signal);
}
process.exit(result.status ?? 1);
