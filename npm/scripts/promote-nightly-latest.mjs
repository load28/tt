import { setTimeout as sleep } from "node:timers/promises";
import { execFileSync } from "node:child_process";
import { appendFileSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

const registry = "https://registry.npmjs.org/";
const root = new URL("../../", import.meta.url);
const readJson = path => JSON.parse(readFileSync(path, "utf8"));
const platforms = readJson(new URL("npm/tt-lang/platforms.json", root));
const launcher = readJson(new URL("npm/tt-lang/package.json", root)).name;
const scaffold = readJson(new URL("packages/create-tt/package.json", root)).name;
const unplugin = readJson(new URL("integrations/unplugin/package.json", root)).name;

export function validateCandidate(run, metadata, repository) {
  if (run.repository?.full_name !== repository || run.path !== ".github/workflows/ci.yml" ||
      run.status !== "completed" || run.conclusion !== "success" || run.head_branch !== "main" ||
      !["schedule", "workflow_dispatch"].includes(run.event)) {
    throw new Error("Candidate must be a successful scheduled or manual main CI run in this repository");
  }
  if (metadata.sourceBranch !== "main" || metadata.sourceSha !== run.head_sha ||
      !/^[a-f0-9]{40}$/.test(metadata.sourceSha) || metadata.npmTag !== "next" ||
      !/^\d+\.\d+\.\d+-dev\.\d{8}\.\d+$/.test(metadata.version) ||
      !/^\d+\.\d+\.\d+-next\./.test(metadata.unpluginVersion) ||
      !metadata.unpluginVersion.endsWith(`-next.${metadata.version.replace("-dev.", ".")}`)) {
    throw new Error("Nightly metadata does not match the successful CI run");
  }
}

export function packagesFor(metadata) {
  return [
    ...Object.values(platforms).map(platform => platform.package), launcher, scaffold,
  ].map(name => ({ name, version: metadata.version, sourceSha: metadata.sourceSha }))
    .concat({ name: unplugin, version: metadata.unpluginVersion });
}

export async function promote(metadata, { readPackage, setTag, report, wait = sleep, log = console.warn }, apply) {
  const packages = packagesFor(metadata);
  const plan = [];
  // Validate every package before the first registry write. Independent
  // unplugin releases retain their own version and original commit identity.
  for (const pkg of packages) {
    const published = await readPackage(pkg.name, pkg.version);
    if (published.version !== pkg.version || !published.dist?.integrity ||
        (pkg.sourceSha && published.gitHead && published.gitHead !== pkg.sourceSha)) {
      throw new Error(`Published package does not match the candidate: ${pkg.name}@${pkg.version}`);
    }
    if (published["dist-tags"]?.next !== pkg.version) {
      throw new Error(`${pkg.name}@next changed or is not published; wait for Nightly publication and run again`);
    }
    if (pkg.name === launcher) {
      for (const { package: name } of Object.values(platforms)) {
        if (published.optionalDependencies?.[name] !== metadata.version) {
          throw new Error(`Launcher dependency does not match Nightly: ${name}`);
        }
      }
    }
    plan.push({ ...pkg, previous: published["dist-tags"].latest ?? null });
  }
  await report(plan);
  if (apply) {
    for (const pkg of plan) {
      if (pkg.previous !== pkg.version) await setTag(pkg.name, pkg.version, "latest");
    }
    // Registry reads may lag behind successful tag writes. Retry only reads,
    // in shared rounds, so multiple stale packages do not multiply the wait.
    const delays = [2000, 4000, 8000, 16000, 30000];
    let pending = plan;
    for (let attempt = 0; ; attempt++) {
      const mismatches = [];
      for (const pkg of pending) {
        const published = await readPackage(pkg.name, pkg.version);
        const observed = published["dist-tags"]?.latest ?? "(absent)";
        if (observed !== pkg.version) mismatches.push({ ...pkg, observed });
      }
      if (mismatches.length === 0) break;
      const details = mismatches.map(pkg => `${pkg.name}: expected ${pkg.version}, observed ${pkg.observed}`).join("; ");
      if (attempt === delays.length) {
        throw new Error(`latest verification failed after ${attempt + 1} reads: ${details}. Tag writes already completed; inspect the registry before rerunning.`);
      }
      log(`Waiting ${delays[attempt] / 1000}s for registry propagation (read ${attempt + 1}): ${details}`);
      await wait(delays[attempt]);
      pending = mismatches;
    }
  }
  return plan;
}

function command(name, args) {
  return execFileSync(name, args, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
}

function loadCandidate(repository, runId) {
  if (!/^\d+$/.test(String(runId))) throw new Error("Invalid CI run ID");
  const run = JSON.parse(command("gh", ["api", `repos/${repository}/actions/runs/${runId}`]));
  const dir = mkdtempSync(join(tmpdir(), "tt-nightly-latest-"));
  try {
    command("gh", ["run", "download", String(runId), "--repo", repository, "--name", "release-metadata", "--dir", dir]);
    const metadata = readJson(join(dir, "release-metadata.json"));
    validateCandidate(run, metadata, repository);
    return { runId, metadata };
  } finally { rmSync(dir, { recursive: true, force: true }); }
}

async function main() {
  const [mode, planFile] = process.argv.slice(2);
  const repository = process.env.GITHUB_REPOSITORY;
  if (!["prepare", "apply"].includes(mode) || !planFile || !repository ||
      process.env.GITHUB_REF !== "refs/heads/main" || process.env.GITHUB_EVENT_NAME !== "workflow_dispatch") {
    throw new Error("Use prepare/apply with a plan file from a manual main workflow only");
  }
  let candidate;
  if (mode === "prepare") {
    const runs = JSON.parse(command("gh", ["run", "list", "--repo", repository, "--workflow", "ci.yml", "--branch", "main", "--status", "success", "--limit", "100", "--json", "databaseId,event"]));
    const run = runs.find(run => ["schedule", "workflow_dispatch"].includes(run.event));
    if (!run) throw new Error("No successful main Nightly CI run found");
    candidate = loadCandidate(repository, run.databaseId);
  } else {
    const saved = readJson(planFile);
    candidate = loadCandidate(repository, saved.runId);
    if (JSON.stringify(candidate) !== JSON.stringify(saved)) throw new Error("Approved candidate metadata changed");
  }
  await promote(candidate.metadata, {
    readPackage: (name, version) => JSON.parse(command("npm", ["view", `${name}@${version}`, "--json", "--prefer-online", "--registry", registry])),
    setTag: (name, version, tag) => command("npm", ["dist-tag", "add", `${name}@${version}`, tag, "--registry", registry]),
    report: plan => {
      const summary = `## Temporary Nightly → latest\n\nCI: https://github.com/${repository}/actions/runs/${candidate.runId}\n\nSource: ${candidate.metadata.sourceSha}\n\n| Package | Previous latest | Target |\n| --- | --- | --- |\n` +
        plan.map(p => `| ${p.name} | ${p.previous ?? "(absent)"} | ${p.version} |`).join("\n") + "\n";
      console.log(summary);
      if (process.env.GITHUB_STEP_SUMMARY) appendFileSync(process.env.GITHUB_STEP_SUMMARY, summary);
    },
  }, mode === "apply");
  if (mode === "prepare") writeFileSync(planFile, JSON.stringify(candidate));
}

if (process.argv[1] && pathToFileURL(process.argv[1]).href === import.meta.url) {
  main().catch(error => { console.error(error.message); process.exitCode = 1; });
}
