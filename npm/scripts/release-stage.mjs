import { pathToFileURL } from "node:url";

const LINE = /^(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
const VERSION = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-(beta|rc))?$/;

export function releaseStageVersion(stage, line, currentVersion = "") {
  const parsedLine = LINE.exec(line);
  if (!parsedLine) throw new Error(`release line must be X.Y: ${line}`);
  const prefix = `${parsedLine[1]}.${parsedLine[2]}`;

  if (stage === "beta") {
    if (currentVersion) throw new Error(`release-${line} already exists at ${currentVersion}`);
    return `${prefix}.0-beta`;
  }

  const current = VERSION.exec(currentVersion);
  if (!current || `${current[1]}.${current[2]}` !== prefix) {
    throw new Error(`current version must belong to release-${line}: ${currentVersion || "<missing>"}`);
  }
  const patch = Number(current[3]);
  const prerelease = current[4] ?? null;

  if (stage === "rc" && patch === 0 && prerelease === "beta") return `${prefix}.1-rc`;
  if (stage === "stable" && patch === 1 && prerelease === "rc") return `${prefix}.2`;
  if (stage === "patch" && patch >= 2 && prerelease === null) return `${prefix}.${patch + 1}`;
  throw new Error(`cannot advance ${currentVersion} to ${stage}`);
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    const [stage, line, currentVersion = ""] = process.argv.slice(2);
    if (!stage || !line) throw new Error("usage: release-stage.mjs <beta|rc|stable|patch> <X.Y> [current-version]");
    console.log(releaseStageVersion(stage, line, currentVersion));
  } catch (error) {
    console.error(`release-stage: ${error.message}`);
    process.exit(1);
  }
}
