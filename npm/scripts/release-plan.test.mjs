import assert from "node:assert/strict";
import test from "node:test";

import { planPendingRelease, planRelease } from "./release-plan.mjs";

const tags = [
  { name: "v0.2.0", sha: "stable-020" },
  { name: "dev-v0.3.0-dev.6.20260823.155239.15.1", sha: "dev-6" },
  { name: "dev-v0.3.0-dev.7", sha: "dev-7" },
];

test("development increments the latest Dev number and accepts an explicit core", () => {
  assert.deepEqual(planRelease({ channel: "development", sourceSha: "work", tags, branches: [] }), {
    version: "0.3.0-dev.8", branch: "release/dev-0.3.0-dev.8", sourceSha: "work", resume: false,
  });
  assert.equal(planRelease({ channel: "development", requested: "0.4.0", sourceSha: "work", tags, branches: [] }).version, "0.4.0-dev.1");
});

test("development increments patch after the latest stable release", () => {
  const plan = planRelease({ channel: "development", sourceSha: "work", tags: [{ name: "v1.2.3", sha: "stable" }], branches: [] });
  assert.equal(plan.version, "1.2.4-dev.1");
});

test("a pending branch is resumed without allocating another version", () => {
  const plan = planRelease({
    channel: "development", sourceSha: "work", tags, branches: [{ name: "release/dev-0.3.0-dev.8", sha: "pending" }],
  });
  assert.deepEqual(plan, { version: "0.3.0-dev.8", branch: "release/dev-0.3.0-dev.8", sourceSha: "pending", resume: true });
});

test("production promotes the latest unpromoted same-core Dev commit", () => {
  const plan = planRelease({ channel: "production", requested: "0.3.0", sourceSha: "newer-main", tags, branches: [] });
  assert.deepEqual(plan, {
    version: "0.3.0", branch: "release/v0.3.0", sourceSha: "dev-7", devTag: "dev-v0.3.0-dev.7", resume: false,
  });
});

test("approval selects an existing branch and never allocates a version", () => {
  const branches = [{ name: "release/dev-0.3.0-dev.8", sha: "ready" }];
  assert.deepEqual(planPendingRelease({ channel: "development", requested: "0.3.0", branches }), {
    version: "0.3.0-dev.8", branch: "release/dev-0.3.0-dev.8", sourceSha: "ready", resume: true,
  });
  assert.throws(() => planPendingRelease({ channel: "production", branches }), /no pending/);
});

test("stable cores cannot receive another Dev and cannot be promoted twice", () => {
  const released = [...tags, { name: "v0.3.0", sha: "prod" }];
  assert.throws(() => planRelease({ channel: "development", requested: "0.3.0", sourceSha: "work", tags: released, branches: [] }), /already exists/);
  assert.throws(() => planRelease({ channel: "production", requested: "0.3.0", sourceSha: "dev-7", tags: released, branches: [] }), /no successful/);
});
