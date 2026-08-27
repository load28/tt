/* --------------------------------------------------------------------------
 * @load28/tt-lang local development mode — created by `scripts/setup` in the TT
 * repository, absent from published installs.
 *
 * `tt-dev.local.json` beside this file (written by setup, gitignored) marks
 * the package as a local `file:` install and names the TT repository root,
 * from which the one thing this layer supplies is derived:
 *
 *   ttc  <root>/target/release/ttc            (the release build)
 *
 * TypeScript is *not* part of it: ttc resolves TypeScript 7 from the
 * consuming project's own `node_modules`, and there is no second way to
 * name one (`src/typescript/toolchain.rs`). A development install differs
 * from a published one in which ttc runs, and in nothing else.
 *
 * This whole layer is temporary (docs/tasks/TASK-090): once TT ships a
 * verified TypeScript inside its own package, delete this module together
 * with scripts/setup and .tt-dev/.
 * ----------------------------------------------------------------------- */
"use strict";

const fs = require("node:fs");
const path = require("node:path");

const DEV_CONFIG = path.join(__dirname, "tt-dev.local.json");

/** The parsed JSON object of `file`, or null when missing/unreadable. */
function readConfig(file) {
  try {
    const parsed = JSON.parse(fs.readFileSync(file, "utf8"));
    return parsed && typeof parsed === "object" ? parsed : null;
  } catch {
    return null;
  }
}

/**
 * The local development environment `scripts/setup` configured, or null when
 * this is a regular published install (no `tt-dev.local.json`).
 *
 * Throws when the config exists but the binary it names is gone — running
 * some other ttc silently would be worse than failing with the fix.
 *
 * @returns {{ binary: string } | null}
 */
function devEnvironment() {
  const config = readConfig(DEV_CONFIG);
  const root = config && typeof config.root === "string" ? config.root : null;
  if (root === null) return null;

  const exe = process.platform === "win32" ? "ttc.exe" : "ttc";
  const binary = path.join(root, "target", "release", exe);
  if (!fs.existsSync(binary)) {
    throw new Error(
      `@load28/tt-lang: this is a local development install (${DEV_CONFIG}), ` +
        `but ${binary} does not exist. Run ./scripts/setup in ${root}, ` +
        `then reinstall this package.`,
    );
  }
  return { binary };
}

module.exports = { devEnvironment };
