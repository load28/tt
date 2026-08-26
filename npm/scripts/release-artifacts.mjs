import { pathToFileURL } from "node:url";

const VERSION = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-(dev\.([1-9]\d*)|rc))?$/;
const STABLE = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
const STAMP = /^(\d{4})(\d{2})(\d{2})(\d{2})(\d{2})(\d{2})$/;

export function nightlyVersion(sourceVersion, timestamp) {
  const source = VERSION.exec(sourceVersion);
  const stamp = requireTimestamp(timestamp);
  if (!source) throw new Error(`source version must be SemVer: ${sourceVersion}`);
  return `${source[1]}.${source[2]}.${source[3]}-dev.${stamp[1]}${stamp[2]}${stamp[3]}`;
}

export function releaseArtifacts({ compilerVersion, extensionBase, unpluginBase, timestamp }) {
  const compiler = VERSION.exec(compilerVersion);
  const extension = requireStable("extension", extensionBase);
  const unplugin = requireStable("unplugin", unpluginBase);
  const stamp = requireTimestamp(timestamp);
  if (!compiler) throw new Error(`compiler version must be a supported release version: ${compilerVersion}`);

  const prerelease = compiler[4] ?? null;
  const npmTag = prerelease?.startsWith("dev.") ? "next" : prerelease ?? "latest";
  const suffix = `${npmTag}.${compiler[1]}.${compiler[2]}.${compiler[3]}` +
    (compiler[5] ? `.${compiler[5]}` : "");

  return {
    version: compilerVersion,
    npmTag,
    unpluginVersion: prerelease ? `${unplugin[1]}.${unplugin[2]}.${unplugin[3]}-${suffix}` : unpluginBase,
    vscodeVersion: prerelease
      ? `${extension[1]}.${stamp[1].slice(2)}${stamp[2]}${stamp[3]}.${stamp[4]}${stamp[5]}${stamp[6]}`
      : extensionBase,
  };
}

function requireStable(name, version) {
  const parsed = STABLE.exec(version);
  if (!parsed) throw new Error(`${name} base must be X.Y.Z: ${version}`);
  return parsed;
}

function requireTimestamp(timestamp) {
  const stamp = STAMP.exec(timestamp);
  if (!stamp) throw new Error(`invalid UTC timestamp: ${timestamp}`);
  const [, year, month, day, hour, minute, second] = stamp;
  const date = new Date(`${year}-${month}-${day}T${hour}:${minute}:${second}Z`);
  if (Number.isNaN(date.valueOf()) || date.toISOString() !== `${year}-${month}-${day}T${hour}:${minute}:${second}.000Z`) {
    throw new Error(`invalid UTC timestamp: ${timestamp}`);
  }
  return stamp;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    const [command, ...args] = process.argv.slice(2);
    if (command === "nightly" && args.length === 2) {
      console.log(nightlyVersion(args[0], args[1]));
    } else if (command === "describe" && args.length === 4) {
      const result = releaseArtifacts({
        compilerVersion: args[0], extensionBase: args[1], unpluginBase: args[2], timestamp: args[3],
      });
      for (const [key, value] of Object.entries(result)) console.log(`${key.replace(/[A-Z]/g, c => `_${c.toLowerCase()}`)}=${value}`);
    } else {
      throw new Error("usage: release-artifacts.mjs nightly <source-version> <timestamp> | describe <version> <extension-base> <unplugin-base> <timestamp>");
    }
  } catch (error) {
    console.error(`release-artifacts: ${error.message}`);
    process.exit(1);
  }
}
