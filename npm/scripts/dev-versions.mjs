/* --------------------------------------------------------------------------
 * dev-versions.mjs — derive immutable registry versions for a development run.
 *
 *   node dev-versions.mjs \
 *     <compiler-dev-version> <extension-base> <unplugin-base> <YYYYMMDDHHMMSS>
 *
 * Prints GitHub Actions output lines. npm accepts prerelease identifiers, while
 * VS Code extension manifests use three numeric components, so the same UTC
 * instant has two artifact-specific representations.
 * ----------------------------------------------------------------------- */

import { pathToFileURL } from "node:url";

const RELEASE = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
const DEVELOPMENT = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)-dev\.([1-9]\d*)$/;
const STAMP = /^(\d{4})(\d{2})(\d{2})(\d{2})(\d{2})(\d{2})$/;
export function devVersions({
  compilerBase,
  extensionBase,
  unpluginBase,
  timestamp,
}) {
  const compiler = DEVELOPMENT.exec(compilerBase);
  if (!compiler) {
    throw new Error(`compiler version must be major.minor.patch-dev.N: ${compilerBase}`);
  }
  const extension = requireRelease("extension", extensionBase);
  const unplugin = requireRelease("unplugin", unpluginBase);

  const stamp = STAMP.exec(timestamp);
  if (!stamp || !isUtcTimestamp(stamp)) {
    throw new Error(`invalid UTC timestamp: ${timestamp}`);
  }
  const [, year, month, day, hour, minute, second] = stamp;
  const time = `${hour}${minute}${second}`;
  const dependentSuffix = `dev.${compiler[1]}.${compiler[2]}.${compiler[3]}.${compiler[4]}`;

  return {
    timestamp,
    npmVersion: compilerBase,
    unpluginVersion: `${unplugin[1]}.${unplugin[2]}.${unplugin[3]}-${dependentSuffix}`,
    // The release commit timestamp is immutable, so retries reuse this VSIX version.
    vscodeVersion: `${extension[1]}.${year.slice(2)}${month}${day}.${time}`,
  };
}

function requireRelease(name, version) {
  const parsed = RELEASE.exec(version);
  if (!parsed) throw new Error(`${name} base must be major.minor.patch: ${version}`);
  return parsed;
}

function isUtcTimestamp(match) {
  const [, year, month, day, hour, minute, second] = match;
  const date = new Date(`${year}-${month}-${day}T${hour}:${minute}:${second}Z`);
  return !Number.isNaN(date.valueOf()) && date.toISOString() === `${year}-${month}-${day}T${hour}:${minute}:${second}.000Z`;
}

function main(args) {
  if (args.length !== 4) {
    throw new Error(
      "usage: dev-versions.mjs <compiler-dev-version> <extension-base> " +
        "<unplugin-base> <YYYYMMDDHHMMSS>",
    );
  }
  const versions = devVersions({
    compilerBase: args[0],
    extensionBase: args[1],
    unpluginBase: args[2],
    timestamp: args[3],
  });
  console.log(`timestamp=${versions.timestamp}`);
  console.log(`npm_version=${versions.npmVersion}`);
  console.log(`unplugin_version=${versions.unpluginVersion}`);
  console.log(`vscode_version=${versions.vscodeVersion}`);
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    main(process.argv.slice(2));
  } catch (error) {
    console.error(`dev-versions: ${error.message}`);
    process.exit(1);
  }
}
