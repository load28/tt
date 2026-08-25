import { execFileSync } from "node:child_process";
import { pathToFileURL } from "node:url";

const CORE = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
const DEV = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)-dev\.([1-9]\d*)$/;
const DEV_TAG = /^dev-v((?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)-dev\.([1-9]\d*))(?:\..+)?$/;

export function planRelease({ channel, requested = "", mainSha, tags = [], branches = [] }) {
  if (!["development", "production"].includes(channel)) throw new Error(`unknown channel: ${channel}`);
  if (requested && !CORE.test(requested)) throw new Error(`version must be X.Y.Z: ${requested}`);

  const pending = matchingPendingBranches(channel, requested, branches);
  if (pending.length > 1) throw new Error(`multiple pending ${channel} release branches match; specify or clean one`);
  if (pending.length === 1) {
    return pendingPlan(channel, pending[0]);
  }

  const releases = parseReleaseTags(tags);
  if (channel === "development") return planDevelopment(requested, mainSha, releases);
  return planProduction(requested, mainSha, releases);
}

export function planPendingRelease({ channel, requested = "", branches = [] }) {
  if (!["development", "production"].includes(channel)) throw new Error(`unknown channel: ${channel}`);
  if (requested && !CORE.test(requested)) throw new Error(`version must be X.Y.Z: ${requested}`);
  const pending = matchingPendingBranches(channel, requested, branches);
  if (pending.length === 0) throw new Error(`no pending ${channel} release branch matches`);
  if (pending.length > 1) throw new Error(`multiple pending ${channel} release branches match; specify one`);
  return pendingPlan(channel, pending[0]);
}

function matchingPendingBranches(channel, requested, branches) {
  const prefix = channel === "development" ? "release/dev-" : "release/v";
  return branches.filter((branch) => branch.name.startsWith(prefix)).filter((branch) => {
    if (!requested) return true;
    return channel === "development"
      ? branch.name.startsWith(`${prefix}${requested}-dev.`)
      : branch.name === `${prefix}${requested}`;
  });
}

function pendingPlan(channel, branch) {
  const prefix = channel === "development" ? "release/dev-" : "release/v";
  const version = branch.name.slice(prefix.length);
  requireChannelVersion(channel, version);
  return { version, branch: branch.name, sourceSha: branch.sha, resume: true };
}

function planDevelopment(requested, mainSha, releases) {
  const stableCores = new Set(releases.filter((item) => item.channel === "production").map((item) => item.core));
  let core;
  let number;
  if (requested) {
    if (stableCores.has(requested)) throw new Error(`stable ${requested} already exists`);
    core = requested;
    number = highestDev(releases, core) + 1;
  } else {
    const latest = [...releases].sort(compareRelease).at(-1);
    if (!latest) throw new Error("no successful release exists; provide an initial X.Y.Z version");
    if (latest.channel === "development") {
      core = latest.core;
      number = highestDev(releases, core) + 1;
    } else {
      const [major, minor, patch] = latest.core.split(".").map(Number);
      core = `${major}.${minor}.${patch + 1}`;
      number = 1;
    }
  }
  const version = `${core}-dev.${number}`;
  return { version, branch: `release/dev-${version}`, sourceSha: mainSha, resume: false };
}

function planProduction(requested, _mainSha, releases) {
  const stableCores = new Set(releases.filter((item) => item.channel === "production").map((item) => item.core));
  const candidates = releases.filter((item) =>
    item.channel === "development" && !stableCores.has(item.core) && (!requested || item.core === requested),
  );
  const development = candidates.sort(compareRelease).at(-1);
  if (!development) throw new Error(`no successful unpromoted Dev release${requested ? ` for ${requested}` : ''}`);
  return {
    version: development.core,
    branch: `release/v${development.core}`,
    sourceSha: development.sha,
    devTag: development.tag,
    resume: false,
  };
}

export function parseReleaseTags(tags) {
  const releases = [];
  for (const { name, sha } of tags) {
    const dev = DEV_TAG.exec(name);
    if (dev) {
      const parsed = DEV.exec(dev[1]);
      releases.push({
        channel: "development",
        core: `${parsed[1]}.${parsed[2]}.${parsed[3]}`,
        number: Number(parsed[4]),
        sha,
        tag: name,
      });
      continue;
    }
    const stable = /^v(.+)$/.exec(name);
    if (stable && CORE.test(stable[1])) {
      releases.push({ channel: "production", core: stable[1], number: 0, sha, tag: name });
    }
  }
  return releases;
}

function highestDev(releases, core) {
  return Math.max(
    0,
    ...releases
      .filter((item) => item.channel === "development" && item.core === core)
      .map((item) => item.number),
  );
}

function compareRelease(left, right) {
  const core = compareCore(left.core, right.core);
  if (core !== 0) return core;
  if (left.channel !== right.channel) return left.channel === "production" ? 1 : -1;
  return left.number - right.number || left.tag.localeCompare(right.tag);
}

function compareCore(left, right) {
  const a = left.split(".").map(Number);
  const b = right.split(".").map(Number);
  return a[0] - b[0] || a[1] - b[1] || a[2] - b[2];
}

function requireChannelVersion(channel, version) {
  const pattern = channel === "development" ? DEV : CORE;
  if (!pattern.test(version)) throw new Error(`pending branch has an invalid ${channel} version: ${version}`);
}

function git(...args) {
  return execFileSync("git", args, { encoding: "utf8" }).trim();
}

function refs(prefix) {
  const output = git("for-each-ref", "--format=%(refname:short)", prefix);
  if (!output) return [];
  return output.split("\n").map((ref) => ({ ref, sha: git("rev-parse", `${ref}^{commit}`) }));
}

function main(args) {
  const approve = args[0] === "approve-dev" || args[0] === "approve-prod";
  const channel = ["dev", "approve-dev"].includes(args[0])
    ? "development"
    : ["prod", "approve-prod"].includes(args[0])
      ? "production"
      : args[0];
  const requested = args[1] ?? "";
  const tags = refs("refs/tags").map(({ ref, sha }) => ({ name: ref, sha }));
  const branches = refs("refs/remotes/origin/release").map(({ ref, sha }) => ({
    name: ref.replace(/^origin\//, ""),
    sha,
  }));
  const plan = approve
    ? planPendingRelease({ channel, requested, branches })
    : planRelease({
        channel,
        requested,
        mainSha: git("rev-parse", "origin/main^{commit}"),
        tags,
        branches,
      });
  process.stdout.write(`${JSON.stringify(plan, null, 2)}\n`);
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    main(process.argv.slice(2));
  } catch (error) {
    console.error(`release-plan: ${error.message}`);
    process.exit(1);
  }
}
