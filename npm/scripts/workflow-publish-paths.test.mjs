import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const workflows = ["dev-release.yml", "release.yml"].map((name) =>
  readFileSync(new URL(`../../.github/workflows/${name}`, import.meta.url), "utf8"),
);

const packageDirectories = ["npm/tt-lang", "integrations/unplugin", "packages/create-tt"];

test("release workflows publish fixed package directories as explicit local paths", () => {
  for (const workflow of workflows) {
    for (const directory of packageDirectories) {
      assert.match(workflow, new RegExp(`npm publish \\./${directory}(?: |$)`));
      assert.doesNotMatch(workflow, new RegExp(`npm publish ${directory}(?: |$)`));
    }
  }
});

test("release workflows install targets into the pinned Rust toolchain", () => {
  for (const workflow of workflows) {
    assert.match(workflow, /run: rustup target add \$\{\{ matrix\.target \}\}/);
  }
});
