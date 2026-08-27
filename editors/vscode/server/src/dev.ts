/* --------------------------------------------------------------------------
 * Local development toolchain — the editor half of `scripts/setup`.
 *
 * Setup records its TypeScript toolchain choice in `.tt-dev/toolchain.json`
 * at the TT repository root, and the npm launcher (npm/tt-lang/dev.js) hands
 * it to ttc as TTC_TSGO_* variables on the child process. This module does
 * the same for every ttc this server spawns, so the CLI and the extension
 * always drive the identical TypeScript toolchain:
 *
 * - `ttcSpawnEnv(compiler)` walks up from the compiler binary to the nearest
 *   `.tt-dev/toolchain.json` (the resolved compiler is the repository's
 *   `target/release/ttc`, so the config sits two directories up) and
 *   computes the TTC_TSGO_* additions. A published ttc, an "npm" toolchain
 *   (TypeScript 7 from the project's node_modules) or an unbuilt checkout
 *   add nothing — ttc then resolves TypeScript by its own documented order.
 * Which ttc a project provides is not this module's question — the installed
 * package answers that for every install shape, development ones included
 * (install.ts).
 *
 * The variables live only on the spawned processes — the user's shell and
 * VSCode's own environment are never modified. The whole layer is temporary
 * (docs/tasks/TASK-090) and disappears with `.tt-dev/`.
 * ----------------------------------------------------------------------- */
import * as fs from "node:fs";
import * as path from "node:path";

/** tsgo's path inside a built typescript-go tree (src/typescript/native.rs). */
const BIN_IN_TREE = ["built", "local", "tsgo"];
/** The JS API client's path inside a built typescript-go tree. */
const API_IN_TREE = ["_packages", "native-preview", "dist", "api", "sync", "api.js"];

/** The parsed JSON object of `file`, or null when missing/unreadable. */
function readConfig(file: string): Record<string, unknown> | null {
  try {
    const parsed: unknown = JSON.parse(fs.readFileSync(file, "utf8"));
    return typeof parsed === "object" && parsed !== null
      ? (parsed as Record<string, unknown>)
      : null;
  } catch {
    return null;
  }
}

function exists(file: string): boolean {
  try {
    return fs.existsSync(file);
  } catch {
    return false;
  }
}

/**
 * The TTC_TSGO_* additions for the toolchain configured in the
 * `.tt-dev/toolchain.json` nearest above `compiler`, or null when there is
 * nothing to add (no config, an "npm" toolchain, an unbuilt checkout, or a
 * bare `ttc` from PATH — no directory to walk from).
 */
export function toolchainEnv(compiler: string): Record<string, string> | null {
  if (!compiler.includes("/") && !compiler.includes(path.sep)) return null;
  let dir = path.resolve(path.dirname(compiler));
  for (;;) {
    const config = readConfig(path.join(dir, ".tt-dev", "toolchain.json"));
    if (config !== null) {
      // A config predating the "kind" field named a checkout root only.
      const kind = typeof config.kind === "string" ? config.kind : "checkout";
      const root = typeof config.root === "string" ? config.root : null;
      if (kind !== "checkout" || root === null) return null;
      const bin = path.join(root, ...BIN_IN_TREE);
      const api = path.join(root, ...API_IN_TREE);
      // An unbuilt checkout must not be named: TTC_TSGO_ROOT would stop
      // ttc's own fallback resolution cold.
      if (!exists(bin) || !exists(api)) return null;
      return { TTC_TSGO_ROOT: root, TTC_TSGO_BIN: bin, TTC_TSGO_API: api };
    }
    const parent = path.dirname(dir);
    if (parent === dir) return null;
    dir = parent;
  }
}

/**
 * The environment for spawning `compiler`: the process environment plus the
 * toolchain variables, or undefined (inherit untouched) when there are none.
 */
export function ttcSpawnEnv(compiler: string): NodeJS.ProcessEnv | undefined {
  const extra = toolchainEnv(compiler);
  return extra === null ? undefined : { ...process.env, ...extra };
}
