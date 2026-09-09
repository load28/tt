import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { packagesFor, promote, validateCandidate } from "./promote-nightly-latest.mjs";

const metadata = { sourceBranch: "main", sourceSha: "a".repeat(40), npmTag: "next", version: "1.0.0-dev.20260909.333", unpluginVersion: "0.1.0-next.1.0.0.20260909.333" };
const run = { repository: { full_name: "load28/tt" }, path: ".github/workflows/ci.yml", status: "completed", conclusion: "success", head_branch: "main", head_sha: metadata.sourceSha, event: "schedule" };
function fixture() {
  const packages = packagesFor(metadata);
  const state = new Map(packages.map(p => [p.name, {
    version: p.version, gitHead: p.sourceSha ?? "b".repeat(40), dist: { integrity: "sha512-example" },
    "dist-tags": { next: p.version, latest: "0.0.1" },
    optionalDependencies: Object.fromEntries(packages.filter(p => p.name.startsWith("@openload28/tt-lang-")).map(p => [p.name, p.version])),
  }]));
  const writes = [];
  return { state, writes, readPackage: async name => structuredClone(state.get(name)),
    setTag: async (name, version, tag) => { writes.push({ name, version, tag }); state.get(name)["dist-tags"][tag] = version; },
    report: async () => {},
  };
}

test("only successful main Nightly CI metadata can be promoted", () => {
  validateCandidate(run, metadata, "load28/tt");
  validateCandidate({ ...run, event: "workflow_dispatch" }, metadata, "load28/tt");
  for (const patch of [{ conclusion: "failure" }, { status: "in_progress" }, { event: "push" }, { event: "pull_request" }, { head_branch: "release-1.0" }, { path: ".github/workflows/other.yml" }, { repository: { full_name: "someone/tt" } }]) {
    assert.throws(() => validateCandidate({ ...run, ...patch }, metadata, "load28/tt"));
  }
  for (const patch of [{ sourceSha: "b".repeat(40) }, { npmTag: "latest" }, { sourceBranch: "release-1.0" }, { version: "1.0.1-rc" }, { unpluginVersion: "oops" }, { unpluginVersion: "0.1.0-next.1.0.0.20260908.330" }]) {
    assert.throws(() => validateCandidate(run, { ...metadata, ...patch }, "load28/tt"));
  }
});

test("preflight shows all eight packages without changing tags", async () => {
  const io = fixture();
  const plan = await promote(metadata, io, false);
  assert.equal(plan.length, 8);
  assert.equal(plan.at(-1).version, metadata.unpluginVersion);
  assert.equal(io.writes.length, 0);
});

test("a missing, changed or inconsistent final package prevents every write", async () => {
  for (const corrupt of [
    io => { io.state.get("@openload28/create-tt").version = "missing"; },
    io => { io.state.get("@openload28/create-tt").gitHead = "b".repeat(40); },
    io => { io.state.get("@openload28/unplugin-tt")["dist-tags"].next = "0.2.0"; },
    io => { io.state.get("@openload28/tt-lang").optionalDependencies["@openload28/tt-lang-linux-x64"] = "old"; },
    io => { io.state.get("@openload28/create-tt").dist = {}; },
  ]) {
    const io = fixture(); corrupt(io);
    await assert.rejects(promote(metadata, io, true));
    assert.equal(io.writes.length, 0);
  }
});

test("promotion keeps next and independent versions; a repeated run is a no-op", async () => {
  const io = fixture();
  await promote(metadata, io, true);
  assert.equal(io.writes.length, 8);
  assert.ok(io.writes.every(write => write.tag === "latest"));
  assert.equal(io.state.get("@openload28/unplugin-tt")["dist-tags"].latest, metadata.unpluginVersion);
  assert.equal(io.state.get("@openload28/tt-lang")["dist-tags"].next, metadata.version);
  await promote(metadata, io, true);
  assert.equal(io.writes.length, 8);
});

test("a partial registry failure can be resumed without republishing", async () => {
  const io = fixture();
  const setTag = io.setTag;
  io.setTag = async (...args) => {
    if (io.writes.length === 3) throw new Error("registry unavailable");
    return setTag(...args);
  };
  await assert.rejects(promote(metadata, io, true), /registry unavailable/);
  assert.equal(io.writes.length, 3);
  io.setTag = setTag;
  await promote(metadata, io, true);
  assert.equal(io.writes.length, 8);
});

test("postflight detects tags that the registry did not change", async () => {
  const io = fixture(); io.setTag = async () => {};
  await assert.rejects(promote(metadata, io, true), /latest verification failed/);
});

test("promotion is manual-only and uses the publisher's serialization group", () => {
  const workflow = readFileSync(new URL("../../.github/workflows/promote-nightly-latest.yml", import.meta.url), "utf8");
  const triggers = workflow.split("\non:\n")[1].split("\npermissions:")[0];
  assert.equal(triggers.trim(), "workflow_dispatch:");
  assert.match(workflow, /environment: production/);
  const publisher = readFileSync(new URL("../../.github/workflows/release-publish.yml", import.meta.url), "utf8");
  for (const source of [workflow, publisher]) assert.match(source, /group: npm-release-tags\n\s+cancel-in-progress: false/);
});
