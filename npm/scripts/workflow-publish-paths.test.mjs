import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import test from "node:test";

function workflow(name) {
  return readFileSync(new URL(`../../.github/workflows/${name}`, import.meta.url), "utf8");
}

const ci = workflow("ci.yml");
const create = workflow("new-release-branch.yml");
const sync = workflow("sync-release-branch.yml");
const bump = workflow("bump-release-version.yml");
const publish = workflow("release-publish.yml");

test("CI follows TypeScript's main and release-X.Y branch model", () => {
  assert.match(ci, /push:\n    branches: \[main, "release-\*"\]/);
  assert.match(ci, /pull_request:\n    branches: \[main, "release-\*"\]/);
  assert.match(ci, /merge_group:\n    branches: \[main, "release-\*"\]/);
  assert.match(ci, /schedule:\n    - cron:/);
  assert.match(ci, /name: release build/);
  assert.match(ci, /name: release artifact version/);
  assert.match(ci, /name: release-metadata/);
  assert.match(ci, /release-artifacts\.mjs nightly "\$VERSION" "\$TIMESTAMP" "\$GITHUB_RUN_NUMBER"/);
  assert.match(ci, /actions\/runs\/\$GITHUB_RUN_ID.*--jq \.created_at/);
  assert.match(ci, /needs: release-version/);
  assert.doesNotMatch(ci, /git show -s --format=%cd/);
  // TypeScript is a dependency of this repository, not a checkout CI builds:
  // `npm ci` installs the version package.json pins, and ttc resolves it
  // from node_modules exactly as a consumer project does (TASK-256).
  assert.doesNotMatch(ci, /typescript-go/);
  assert.doesNotMatch(ci, /TTC_TSGO_/);
  assert.match(ci, /name: Install TypeScript\n\s+run: npm ci/);
  assert.match(ci, /name: tsgo type checking \+ vscode extension/);
  assert.doesNotMatch(ci, /^  extension:/m);
  assert.doesNotMatch(ci, /needs: \[[^\]]*extension/);
  assert.doesNotMatch(ci, /npm publish|action-gh-release/);
});

test("release commands mirror TypeScript's create, sync, and bump boundaries", () => {
  for (const command of [create, sync, bump]) {
    assert.match(command, /workflow_dispatch:/);
    assert.match(command, /permissions:\n  contents: read/);
    assert.match(command, /persist-credentials: false/);
    assert.doesNotMatch(command, /npm publish|action-gh-release|git push origin --delete/);
  }

  assert.match(create, /BRANCH="release-\$LINE"/);
  assert.match(create, /git checkout -b "\$BRANCH"/);
  assert.match(create, /VERSION="\$LINE\.0-beta"/);
  assert.match(create, /git push --set-upstream origin "\$BRANCH"/);

  assert.match(sync, /ref: release-\$\{\{ inputs\.line \}\}/);
  assert.match(sync, /git fetch origin main/);
  assert.match(sync, /git merge origin\/main --no-ff/);
  assert.doesNotMatch(sync, /release-version\.mjs|release-bump\.mjs/);

  assert.match(bump, /release-bump\.mjs "\$LINE" "\$CURRENT"/);
  assert.match(bump, /git push origin "HEAD:refs\/heads\/release-\$\{\{ inputs\.line \}\}"/);

  for (const command of [create, sync, bump]) {
    assert.match(command, /actions\/create-github-app-token@v3/);
    assert.match(command, /app-id: \$\{\{ vars\.RELEASE_APP_ID \}\}/);
    assert.match(command, /private-key: \$\{\{ secrets\.RELEASE_APP_PRIVATE_KEY \}\}/);
    assert.match(command, /permission-contents: write/);
    assert.match(command, /GITHUB_APP_TOKEN: \$\{\{/);
    assert.match(command, /git config --local http\.https:\/\/github\.com\/\.extraheader/);
    assert.doesNotMatch(command, /gh workflow run|GITHUB_TOKEN/);
  }
});

test("publishing promotes scheduled or dispatched Nightlies and approved formal releases without manual identifiers", () => {
  assert.match(publish, /workflow_run:/);
  assert.match(publish, /github\.event\.workflow_run\.event == 'schedule'/);
  assert.match(publish, /github\.event\.workflow_run\.event == 'workflow_dispatch'/);
  assert.match(publish, /github\.event\.workflow_run\.head_branch == 'main'/);
  assert.match(publish, /github\.event\.workflow_run\.event == 'push'/);
  assert.match(publish, /startsWith\(github\.event\.workflow_run\.head_branch, 'release-'\)/);
  assert.match(publish, /RUN_ID: \$\{\{ github\.event\.workflow_run\.id \}\}/);
  assert.match(publish, /environment:\n      name: \$\{\{ needs\.verify\.outputs\.environment_name \}\}/);
  assert.match(publish, /ENVIRONMENT_NAME=nightly/);
  assert.match(publish, /ENVIRONMENT_NAME=production/);
  assert.match(publish, /name: Reject a superseded build/);
  assert.match(publish, /gh run list .* --branch "\$SOURCE_BRANCH" --event "\$RUN_EVENT" --limit 1/);
  assert.match(publish, /test "\$RUN_ID" = "\$latest_run_id"/);
  assert.doesNotMatch(publish, /workflow_dispatch:|inputs\.run_id|inputs\.npm_tag/);
  assert.match(publish, /\.conclusion \/tmp\/run\.json\)" = success/);
  assert.match(publish, /gh run download "\$RUN_ID"/);
  assert.match(publish, /printf 'variant E \{ A\(x: number\) \}/);
  assert.doesNotMatch(publish, /printf 'enum E \{ A\(x: number\) \}/);
  assert.match(publish, /npm publish "\.\/\$package_dir" --tag "\$NPM_TAG"/);
  assert.match(publish, /action-gh-release/);
  assert.doesNotMatch(publish, /cargo build|go build|git push origin --delete/);
});

test("publish validates the source branch and immutable npm versions", () => {
  assert.match(publish, /case "\$RUN_EVENT" in schedule\|workflow_dispatch/);
  assert.match(publish, /test "\$RUN_EVENT" = push/);
  assert.match(publish, /test "\$SOURCE_BRANCH" = main/);
  assert.match(publish, /case "\$SOURCE_BRANCH" in release-\*/);
  assert.match(publish, /\*-beta\) EXPECTED_TAG=beta/);
  assert.match(publish, /\*-rc\) EXPECTED_TAG=rc/);
  assert.match(publish, /\*\) EXPECTED_TAG=latest/);
  assert.match(publish, /test "\$NPM_TAG" = "\$EXPECTED_TAG"/);
  assert.match(publish, /run_event=\$RUN_EVENT/);
  assert.match(publish, /npm view "\$name@\$version" version/);
  assert.match(publish, /test -z "\$git_head" \|\| test "\$git_head" = "\$SOURCE_SHA"/);
});

/**
 * A job that runs npm without setting Node up works only by accident — the
 * runner image happens to carry one, at whatever version it happens to
 * carry. The local gate cannot see this class of mistake at all: it shows
 * up only on the hosted runner, and only sometimes. TASK-256 introduced one
 * (`npm ci` replaced a checkout in soak's corpus job, which had never
 * needed Node), so the rule is checked rather than remembered.
 */
test("every workflow job that runs npm sets Node up", () => {
  const dir = new URL("../../.github/workflows/", import.meta.url);
  for (const file of readdirSync(dir).filter((n) => n.endsWith(".yml"))) {
    const text = readFileSync(new URL(file, dir), "utf8");
    const body = text.split(/^jobs:$/m)[1];
    if (!body) continue;
    // Job keys sit at exactly two spaces of indentation.
    const jobs = body.split(/\n(?=  [A-Za-z0-9_-]+:\n)/);
    for (const job of jobs) {
      const name = job.trim().split(":")[0];
      const runsNpm = /\b(npm|npx) /.test(job);
      if (!runsNpm) continue;
      assert.match(
        job,
        /actions\/setup-node/,
        `${file}: job "${name}" runs npm without actions/setup-node`,
      );
    }
  }
});
