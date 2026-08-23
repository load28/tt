/* --------------------------------------------------------------------------
 * @load28/tt-lang local development mode — created by `scripts/setup` in the TT
 * repository, absent from published installs.
 *
 * `tt-dev.local.json` beside this file (written by setup, gitignored) marks
 * the package as a local `file:` install and names the TT repository root.
 * From that one value everything else is derived, never stored twice:
 *
 *   ttc        <root>/target/release/ttc            (the release build)
 *   toolchain  <root>/.tt-dev/toolchain.json        (setup's toolchain choice)
 *
 * A "checkout" toolchain names a typescript-go tree; its tsgo binary and API
 * client paths are computed here and handed to the child ttc process as
 * TTC_TSGO_* environment variables — the user's shell is never touched. An
 * "npm" toolchain injects nothing: ttc resolves TypeScript 7 from the
 * consuming project's own node_modules, exactly as a published ttc would.
 *
 * This whole layer is temporary (docs/tasks/TASK-090): once TT ships a
 * verified TypeScript 7 inside its own package, delete this module together
 * with scripts/setup and .tt-dev/.
 * ----------------------------------------------------------------------- */
"use strict";

const fs = require("node:fs");
const path = require("node:path");

const DEV_CONFIG = path.join(__dirname, "tt-dev.local.json");

/** tsgo's path inside a built typescript-go tree (src/typescript/native.rs). */
const BIN_IN_TREE = "built/local/tsgo";
/** The JS API client's path inside a built typescript-go tree. */
const API_IN_TREE = "_packages/native-preview/dist/api/sync/api.js";

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
 * @returns {{ binary: string, env: Record<string, string> } | null}
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
  return { binary, env: toolchainEnv(root) };
}

/**
 * Environment variables pointing ttc at the toolchain `scripts/setup` chose,
 * read from `<ttRoot>/.tt-dev/toolchain.json`. Empty for the "npm" toolchain
 * (ttc resolves the consuming project's TypeScript itself), for a missing
 * config, and for a checkout whose artifacts are gone — an unbuilt tree
 * named via TTC_TSGO_ROOT would stop ttc's own fallback resolution cold.
 *
 * @param {string} ttRoot
 * @returns {Record<string, string>}
 */
function toolchainEnv(ttRoot) {
  const config = readConfig(path.join(ttRoot, ".tt-dev", "toolchain.json"));
  if (config === null) return {};
  // A config predating the "kind" field named a checkout root only.
  const kind = typeof config.kind === "string" ? config.kind : "checkout";
  const root = typeof config.root === "string" ? config.root : null;
  if (kind !== "checkout" || root === null) return {};

  const bin = path.join(root, ...BIN_IN_TREE.split("/"));
  const api = path.join(root, ...API_IN_TREE.split("/"));
  if (!fs.existsSync(bin) || !fs.existsSync(api)) return {};
  return { TTC_TSGO_ROOT: root, TTC_TSGO_BIN: bin, TTC_TSGO_API: api };
}

module.exports = { devEnvironment };
