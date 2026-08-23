import { pathToFileURL } from "node:url";

const VERSION = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-dev\.([1-9]\d*))?$/;

export function releaseChannel(previousVersion, currentVersion) {
  const previous = parseVersion(previousVersion);
  const current = parseVersion(currentVersion);

  if (previousVersion === currentVersion) return "none";

  const coreOrder = compareCore(current.core, previous.core);
  if (coreOrder < 0) {
    throw new Error(`${currentVersion} does not increase ${previousVersion}`);
  }

  if (current.dev !== null) {
    if (coreOrder === 0 && previous.dev !== null && current.dev <= previous.dev) {
      throw new Error(`${currentVersion} does not increase ${previousVersion}`);
    }
    return "development";
  }

  if (coreOrder === 0 && previous.dev === null) {
    throw new Error(`${currentVersion} does not increase ${previousVersion}`);
  }
  return "production";
}

export function requireDevelopment(version) {
  const parsed = parseVersion(version);
  if (parsed.dev === null) {
    throw new Error(`manual development release requires X.Y.Z-dev.N: ${version}`);
  }
  return "development";
}

export function requireProduction(version) {
  const parsed = parseVersion(version);
  if (parsed.dev !== null) {
    throw new Error(`production release requires X.Y.Z: ${version}`);
  }
  return "production";
}

function parseVersion(version) {
  const match = VERSION.exec(version);
  if (!match) {
    throw new Error(`version must be X.Y.Z or X.Y.Z-dev.N: ${version}`);
  }
  return {
    core: match.slice(1, 4).map(Number),
    dev: match[4] === undefined ? null : Number(match[4]),
  };
}

function compareCore(left, right) {
  for (let index = 0; index < left.length; index += 1) {
    if (left[index] !== right[index]) return left[index] - right[index];
  }
  return 0;
}

function main(args) {
  if (args[0] === "--manual-dev" && args.length === 2) {
    console.log(requireDevelopment(args[1]));
    return;
  }
  if (args[0] === "--production" && args.length === 2) {
    console.log(requireProduction(args[1]));
    return;
  }
  if (args.length !== 2) {
    throw new Error(
      "usage: release-channel.mjs <previous> <current> | --manual-dev <current> | --production <current>",
    );
  }
  console.log(releaseChannel(args[0], args[1]));
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    main(process.argv.slice(2));
  } catch (error) {
    console.error(`release-channel: ${error.message}`);
    process.exit(1);
  }
}
