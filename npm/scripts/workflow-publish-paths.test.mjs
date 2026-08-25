import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

function workflow(name) {
  return readFileSync(new URL(`../../.github/workflows/${name}`, import.meta.url), "utf8");
}

const prepare = [workflow("dev-release.yml"), workflow("release.yml")];
const publish = [workflow("dev-publish.yml"), workflow("release-publish.yml")];

test("preparation and Dev publishing are manual and every release is serialized", () => {
  for (const source of [...prepare, publish[0]]) {
    assert.match(source, /on:\n  workflow_dispatch:/);
    assert.doesNotMatch(source, /workflow_run:|\n  push:/);
  }
  for (const source of [...prepare, ...publish]) {
    assert.match(source, /group: release/);
  }
});

test("prepare workflows build every target with the pinned Rust and tsgo toolchains", () => {
  for (const source of prepare) {
    assert.match(source, /run: rustup target add \$\{\{ matrix\.target \}\}/);
    assert.match(source, /repository: microsoft\/typescript-go/);
    assert.match(source, /ref: c6b013f5706d58582f566df778cc0df2683b58f5/);
    assert.match(source, /go build -o built\/local\/tsgo \.\/cmd\/tsgo/);
    assert.match(source, /TTC_TSGO_ROOT: \$\{\{ github\.workspace \}\}\/typescript-go/);
    assert.match(source, /TTC_REQUIRE_TSGO: "1"/);
    assert.match(source, /statuses\/\$SOURCE_SHA/);
    assert.doesNotMatch(source, /npm publish|action-gh-release/);
  }
});

test("Dev approval requires the prepared SHA and reuses its artifacts", () => {
  assert.match(publish[0], /release-plan\.mjs approve-dev/);
  assert.match(publish[0], /context == "release\/dev-prepare"/);
  assert.match(publish[0], /gh run download/);
});

test("publishing is idempotent and deletes only a completed release branch", () => {
  for (const source of publish) {
    assert.match(source, /npm view "\$name@\$version" version/);
    assert.match(source, /npm publish "\.\/\$package_dir"/);
    assert.match(source, /gitHead/);
    assert.match(source, /publish_(?:dev|latest) npm\/tt-lang/);
    assert.match(source, /publish_(?:dev|latest) integrations\/unplugin/);
    assert.match(source, /publish_(?:dev|latest) packages\/create-tt/);
    assert.match(source, /git push origin --delete/);
  }
});

test("Production preparation promotes Dev, requires its lineage, and opens a main PR", () => {
  assert.match(prepare[1], /release-plan\.mjs prod/);
  assert.match(prepare[1], /git merge-base --is-ancestor origin\/main "\$SOURCE_SHA"/);
  assert.match(prepare[1], /gh pr create/);
  assert.match(prepare[1], /--base main/);
});

test("only merging a prepared Production PR automatically publishes Production", () => {
  assert.match(publish[1], /on:\n  pull_request:\n    types: \[closed\]\n    branches: \[main\]/);
  assert.match(publish[1], /github\.event\.pull_request\.merged == true/);
  assert.match(publish[1], /head\.repo\.full_name == github\.repository/);
  assert.match(publish[1], /startsWith\(github\.event\.pull_request\.head\.ref, 'release\/v'\)/);
  assert.match(publish[1], /context == "release\/prod-prepare"/);
  assert.match(publish[1], /git diff --quiet "\$SOURCE_SHA" "\$MERGE_SHA"/);
  assert.match(publish[1], /gh run download/);
});
