import { pathToFileURL } from "node:url";

const LINE = /^(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
const VERSION = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-(beta|rc))?$/;

export function releaseBumpVersion(line, currentVersion) {
  const parsedLine = LINE.exec(line);
  if (!parsedLine) throw new Error(`release line must be X.Y: ${line}`);
  const prefix = `${parsedLine[1]}.${parsedLine[2]}`;
  const current = VERSION.exec(currentVersion);
  if (!current || `${current[1]}.${current[2]}` !== prefix) {
    throw new Error(`current version must belong to release-${line}: ${currentVersion || "<missing>"}`);
  }

  const patch = Number(current[3]);
  const prerelease = current[4] ?? null;
  if (patch === 0 && prerelease === "beta") return `${prefix}.1-rc`;
  if (patch === 1 && prerelease === "rc") return `${prefix}.2`;
  if (prerelease === null) return `${prefix}.${patch + 1}`;
  throw new Error(`cannot bump ${currentVersion}`);
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    const [line, currentVersion] = process.argv.slice(2);
    if (!line || !currentVersion) throw new Error("usage: release-bump.mjs <X.Y> <current-version>");
    console.log(releaseBumpVersion(line, currentVersion));
  } catch (error) {
    console.error(`release-bump: ${error.message}`);
    process.exit(1);
  }
}
