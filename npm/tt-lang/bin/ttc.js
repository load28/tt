#!/usr/bin/env node
/* --------------------------------------------------------------------------
 * `ttc` launcher — thin veneer over the native binary from the platform
 * package. All arguments, stdio, the exit code and SIGINT/SIGTERM/SIGHUP
 * pass through untouched, so `npx ttc` behaves exactly like a natively
 * installed ttc.
 *
 * In a local development install (`scripts/setup` in the TT repository,
 * then `pnpm add -D file:.../npm/tt-lang`) the binary is the repository's
 * release build instead (dev.js). Nothing else differs: TypeScript comes
 * from the consuming project either way.
 * ----------------------------------------------------------------------- */
"use strict";

const { spawn } = require("node:child_process");

const { binaryPath } = require("../index.js");
const { devEnvironment } = require("../dev.js");

let binary;
try {
  // TTC_BINARY stays the strongest override, exactly as in binaryPath().
  const dev = process.env.TTC_BINARY ? null : devEnvironment();
  binary = dev ? dev.binary : binaryPath();
} catch (error) {
  console.error(error.message);
  process.exit(1);
}

const FORWARDED = ["SIGINT", "SIGTERM", "SIGHUP"];
const child = spawn(binary, process.argv.slice(2), { stdio: "inherit" });
const forward = (signal) => child.kill(signal);
for (const signal of FORWARDED) process.on(signal, forward);
child.on("error", (error) => {
  console.error(`ttc: failed to run ${binary}: ${error.message}`);
  process.exit(1);
});
child.on("exit", (status, signal) => {
  for (const name of FORWARDED) process.removeListener(name, forward);
  if (signal) {
    // Re-raise so the parent observes the same termination signal.
    process.kill(process.pid, signal);
  }
  process.exit(status ?? 1);
});
