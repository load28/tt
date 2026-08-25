import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

function workflow(name) {
  return readFileSync(new URL(`../../.github/workflows/${name}`, import.meta.url), "utf8");
}

const ci = workflow("ci.yml");
const advance = workflow("release.yml");
const publish = workflow("release-publish.yml");

test("CI follows TypeScript's main and release-X.Y branch model", () => {
  assert.match(ci, /push:\n    branches: \[main, "release-\*"\]/);
  assert.match(ci, /pull_request:\n    branches: \[main, "release-\*"\]/);
  assert.match(ci, /merge_group:\n    branches: \[main, "release-\*"\]/);
  assert.match(ci, /name: release build/);
  assert.match(ci, /name: release-metadata/);
  assert.doesNotMatch(ci, /npm publish|action-gh-release/);
});

test("one retained release branch advances through beta, RC, stable, and patch", () => {
  assert.match(advance, /workflow_dispatch:/);
  assert.match(advance, /options: \[beta, rc, stable, patch\]/);
  assert.match(advance, /BRANCH="release-\$LINE"/);
  assert.match(advance, /git merge --no-edit origin\/main/);
  assert.match(advance, /release-stage\.mjs/);
  assert.doesNotMatch(advance, /npm publish|action-gh-release|git push origin --delete/);
});

test("publishing manually promotes one successful CI run without rebuilding", () => {
  assert.match(publish, /workflow_dispatch:/);
  assert.match(publish, /run_id:/);
  assert.match(publish, /options: \[next, beta, rc, latest\]/);
  assert.match(publish, /\.conclusion \/tmp\/run\.json\)" = success/);
  assert.match(publish, /gh run download "\$RUN_ID"/);
  assert.match(publish, /npm publish "\.\/\$package_dir" --tag "\$NPM_TAG"/);
  assert.match(publish, /action-gh-release/);
  assert.doesNotMatch(publish, /cargo build|go build|git push origin --delete/);
});

test("publish validates the source branch and immutable npm versions", () => {
  assert.match(publish, /test "\$SOURCE_BRANCH" = main/);
  assert.match(publish, /case "\$SOURCE_BRANCH" in release-\*/);
  assert.match(publish, /npm view "\$name@\$version" version/);
  assert.match(publish, /test -z "\$git_head" \|\| test "\$git_head" = "\$SOURCE_SHA"/);
});
