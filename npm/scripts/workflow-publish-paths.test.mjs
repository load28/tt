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

test("publishing promotes only scheduled or dispatched Nightlies without manual identifiers", () => {
  assert.match(publish, /workflow_run:/);
  assert.match(publish, /github\.event\.workflow_run\.event == 'schedule'/);
  assert.match(publish, /github\.event\.workflow_run\.event == 'workflow_dispatch'/);
  assert.match(publish, /github\.event\.workflow_run\.head_branch == 'main'/);
  assert.doesNotMatch(publish, /github\.event\.workflow_run\.event == 'push'/);
  assert.doesNotMatch(publish, /startsWith\(github\.event\.workflow_run\.head_branch, 'release-'\)/);
  assert.match(publish, /RUN_ID: \$\{\{ github\.event\.workflow_run\.id \}\}/);
  assert.match(publish, /environment:\n      name: \$\{\{ needs\.verify\.outputs\.environment_name \}\}/);
  assert.match(publish, /ENVIRONMENT_NAME=nightly/);
  assert.doesNotMatch(publish, /ENVIRONMENT_NAME=production/);
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
  // The TypeScript preview extension rides the same promotion: built once
  // in CI, downloaded here, attached as release assets (TASK-258).
  assert.match(ci, /name: release TypeScript preview VSIX/);
  assert.match(ci, /build-ts-preview-vsix\.mjs ts-preview/);
  assert.match(ci, /name: release-ts-preview/);
  assert.match(ci, /release-build, release-vsix, release-ts-preview\]/);
  assert.match(publish, /--name release-ts-preview --dir ts-preview/);
  assert.match(publish, /ts-preview\/\*\.vsix/);
  assert.doesNotMatch(publish, /cargo build|go build|git push origin --delete/);
});

test("publish validates the source branch and immutable npm versions", () => {
  assert.match(publish, /case "\$RUN_EVENT" in schedule\|workflow_dispatch/);
  assert.match(publish, /test "\$SOURCE_BRANCH" = main/);
  assert.match(publish, /test "\$NPM_TAG" = next/);
  assert.doesNotMatch(publish, /EXPECTED_TAG|ENVIRONMENT_NAME=production/);
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

/**
 * A key that must hold a mapping — `env:`, `with:` — with nothing under it
 * is invalid, and GitHub does not fail it the way a broken job fails: the
 * run is created and dies before any step, in zero seconds, listed under
 * the file path instead of the workflow's name. Nothing local sees it,
 * because reading the file is not parsing it (TASK-256 shipped exactly this
 * — a regex removed the last key from an `env:` and left the header).
 *
 * This is not a YAML validator; it is the one shape that editing these
 * files by pattern actually produces.
 */
test("no workflow leaves a mapping key empty", () => {
  const dir = new URL("../../.github/workflows/", import.meta.url);
  const BLOCK_KEYS = /^(\s*)(env|with|outputs|defaults):\s*$/;
  for (const file of readdirSync(dir).filter((n) => n.endsWith(".yml"))) {
    const lines = readFileSync(new URL(file, dir), "utf8").split("\n");
    lines.forEach((line, i) => {
      const match = BLOCK_KEYS.exec(line);
      if (!match) return;
      const indent = match[1].length;
      const next = lines.slice(i + 1).find((l) => l.trim() !== "" && !/^\s*#/.test(l));
      const nextIndent = next === undefined ? -1 : next.length - next.trimStart().length;
      assert.ok(
        next !== undefined && nextIndent > indent,
        `${file}:${i + 1}: \`${match[2]}:\` has nothing under it`,
      );
    });
  }
});
