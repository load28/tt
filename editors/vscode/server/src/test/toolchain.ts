/* What the tests need on the machine, found the same way the server finds
 * it. Every guard here answers "is the tool there?" and nothing else — a
 * skip must mean the tool is missing, never that a feature quietly
 * answered nothing. */
import { execFileSync } from "node:child_process";
import * as path from "node:path";

import { findCompiler, findTsgo as resolveTsgo } from "../ttc";

/** The TT repository root — four levels above `server/out/test`. */
const REPO_ROOT = path.resolve(__dirname, "..", "..", "..", "..", "..");

/**
 * The compiler, resolved exactly as the running server resolves it
 * (`findCompiler`) rather than as a bare PATH name. A guard that only knows
 * one of the server's resolution steps reports "no compiler" wherever the
 * server finds one — and the suite then skips the very cases that would
 * have covered the missing step (TASK-255).
 */
export const COMPILER = findCompiler("", [REPO_ROOT, process.cwd()]);

export function compilerAvailable(): boolean {
  try {
    execFileSync(COMPILER, ["-v"], { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
}

/** The `tsgo` executable, resolved exactly as the running server resolves
 * it, or null when there is none. */
export function findTsgo(): string | null {
  return resolveTsgo([]) || null;
}
