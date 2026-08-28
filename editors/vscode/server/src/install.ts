/* --------------------------------------------------------------------------
 * The compiler a *project* provides.
 *
 * A project consumes tt by installing `@openload28/tt-lang`; that package is what
 * knows where the ttc binary for this machine is (`binaryPath()` in
 * npm/tt-lang/index.js — the `TTC_BINARY` override, a `file:` development
 * install, then the platform package npm picked). The package documents
 * editor servers as one of its callers, so this module *asks* it rather than
 * re-deriving the answer: a second copy of that rule would drift, and would
 * miss the layouts `require.resolve` handles for free (npm hoisting, pnpm's
 * store, a `file:` link).
 *
 * Without this step the extension can only see a compiler built inside the
 * TT repository or one on PATH — so a developer who installed the published
 * packages and the published extension, and nothing else, had an editor with
 * no compiler behind it while `npx ttc` worked (TASK-255).
 * ----------------------------------------------------------------------- */
import { createRequire } from "node:module";
import * as fs from "node:fs";
import * as path from "node:path";

/** The package a project installs to get tt. */
const PACKAGE = ["@openload28", "tt-lang"];

/**
 * The ttc of the `@openload28/tt-lang` installed for one of `workspaceRoots`, or
 * "" when no root has one. Each root is searched the way Node resolves a
 * package: `node_modules` from the root upwards.
 */
export function packageCompiler(workspaceRoots: string[]): string {
  for (const root of workspaceRoots) {
    for (const nodeModules of nodeModulesFrom(root)) {
      const index = path.join(nodeModules, ...PACKAGE, "index.js");
      if (!exists(index)) continue;
      const binary = binaryFrom(index);
      if (binary !== "") return binary;
    }
  }
  return "";
}

/** Every `node_modules` directory from `start` upwards, nearest first. */
function nodeModulesFrom(start: string): string[] {
  const dirs: string[] = [];
  let dir = path.resolve(start);
  for (;;) {
    dirs.push(path.join(dir, "node_modules"));
    const parent = path.dirname(dir);
    if (parent === dir) return dirs;
    dir = parent;
  }
}

/**
 * What the installed package answers when asked for its binary, or "" when
 * it cannot answer — an unsupported platform, optional dependencies that
 * were not installed, or a development install whose build is gone. Each of
 * those is "this project does not provide a compiler", which the caller's
 * next resolution step is entitled to answer differently.
 */
function binaryFrom(index: string): string {
  try {
    const require = createRequire(index);
    const resolved = require.resolve(index);
    // The package is on the user's disk and can be reinstalled while the
    // editor runs; a cached module would keep answering with the old
    // install's binary.
    delete require.cache[resolved];
    delete require.cache[path.join(path.dirname(resolved), "dev.js")];
    const loaded = require(resolved) as { binaryPath?: () => string };
    const binary = loaded.binaryPath?.() ?? "";
    return binary !== "" && exists(binary) ? binary : "";
  } catch {
    return "";
  }
}

function exists(file: string): boolean {
  try {
    return fs.existsSync(file);
  } catch {
    return false;
  }
}
