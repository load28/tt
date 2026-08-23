/* --------------------------------------------------------------------------
 * tt-lang — locate the platform-specific ttc binary.
 *
 * The actual compiler ships in per-platform packages (tt-lang-linux-x64,
 * tt-lang-darwin-arm64, ...) wired up as optionalDependencies: npm installs
 * only the one matching the host. This module finds that binary so tools
 * (the `ttc` bin launcher, unplugin-tt, editor servers) can spawn it
 * directly without paying for an extra node process per call.
 * ----------------------------------------------------------------------- */
"use strict";

const path = require("node:path");
const PLATFORMS = require("./platforms.json");

/** Platforms a prebuilt ttc binary is published for. */
const SUPPORTED = Object.keys(PLATFORMS);

/** `"linux-x64"`-style key for the current process, supported or not. */
function platformKey() {
  return `${process.platform}-${process.arch}`;
}

/** npm package containing the binary for a platform key. */
function platformPackageName(key) {
  return PLATFORMS[key]?.package;
}

/**
 * Absolute path to the ttc binary for this platform.
 *
 * Resolution order: the `TTC_BINARY` environment variable (escape hatch for
 * local builds and tests), then a local development install (`scripts/setup`
 * in the TT repository — see dev.js), then the installed platform package.
 * Throws with install guidance when none is available.
 *
 * @returns {string}
 */
function binaryPath() {
  const override = process.env.TTC_BINARY;
  if (override) return path.resolve(override);

  const dev = require("./dev.js").devEnvironment();
  if (dev) return dev.binary;

  const key = platformKey();
  const exe = process.platform === "win32" ? "ttc.exe" : "ttc";
  const pkg = platformPackageName(key);
  try {
    return require.resolve(`${pkg}/bin/${exe}`);
  } catch {
    const reason = pkg
      ? `the ${pkg} package is not installed — it is an optionalDependency of tt-lang, ` +
        `so this usually means optional dependencies were disabled (--no-optional / ` +
        `--omit=optional) or the lockfile was created on a different platform. ` +
        `Reinstalling tt-lang should fix it.`
      : `no prebuilt ttc binary is published for ${key}. Prebuilt platforms: ` +
        `${SUPPORTED.join(", ")}. You can build from source with ` +
        `"cargo install --git https://github.com/load28/tt" and point the ` +
        `TTC_BINARY environment variable at the resulting binary.`;
    throw new Error(`tt-lang: cannot find the ttc binary: ${reason}`);
  }
}

module.exports = { binaryPath, platformKey, platformPackageName };
