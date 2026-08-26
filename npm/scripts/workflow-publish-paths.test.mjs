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
  assert.match(ci, /schedule:\n    - cron:/);
  assert.match(ci, /name: release build/);
  assert.match(ci, /name: release-metadata/);
  assert.match(ci, /repository: microsoft\/typescript-go/);
  assert.match(ci, /go build -o built\/local\/tsgo \.\/cmd\/tsgo/);
  assert.match(ci, /name: tsgo type checking \+ vscode extension/);
  assert.match(ci, /TTC_TSGO_ROOT: \$\{\{ github\.workspace \}\}\/typescript-go/);
  assert.doesNotMatch(ci, /^  extension:/m);
  assert.doesNotMatch(ci, /npm install .*typescript@7/);
  assert.doesNotMatch(ci, /needs: \[[^\]]*extension/);
  assert.doesNotMatch(ci, /npm publish|action-gh-release/);
});

test("one retained release branch advances through RC, stable, and patch", () => {
  assert.match(advance, /workflow_dispatch:/);
  assert.match(advance, /options: \[rc, stable, patch\]/);
  assert.match(advance, /BRANCH="release-\$LINE"/);
  assert.match(advance, /git checkout -b "\$BRANCH" origin\/main/);
  assert.doesNotMatch(advance, /git merge --no-edit origin\/main/);
  assert.match(advance, /release-stage\.mjs/);
  assert.doesNotMatch(advance, /npm publish|action-gh-release|git push origin --delete/);
});

test("publishing promotes scheduled Nightlies and manually selected formal releases without rebuilding", () => {
  assert.match(publish, /workflow_dispatch:/);
  assert.match(publish, /workflow_run:/);
  assert.match(publish, /github\.event\.workflow_run\.event == 'schedule'/);
  assert.match(publish, /github\.event\.workflow_run\.head_branch == 'main'/);
  assert.match(publish, /run_id:/);
  assert.match(publish, /options: \[next, rc, latest\]/);
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
