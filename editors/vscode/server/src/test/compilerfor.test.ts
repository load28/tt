import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { containingRoot } from "../roots";
import { findCompiler, folderCompiler } from "../ttc";
import { testDir } from "../../../../../scripts/test-dirs.cjs";

/// `tt.compilerPath` is a resource-scoped setting, so two folders in one
/// window may name different compilers. Which folder a document belongs to
/// is what decides which one serves it, and the deepest folder wins so a
/// nested folder's own setting is not shadowed by its parent's.
test("a document is served by the folder that contains it", () => {
  const roots = [path.join("/w", "outer"), path.join("/w", "outer", "inner")];
  assert.equal(
    containingRoot(roots, path.join("/w", "outer", "a.tt")),
    path.join("/w", "outer"),
  );
  assert.equal(
    containingRoot(roots, path.join("/w", "outer", "inner", "b.tt")),
    path.join("/w", "outer", "inner"),
  );
  assert.equal(containingRoot(roots, path.join("/w", "elsewhere", "c.tt")), undefined);
});

test("a folder's compiler path is resolved against that folder", () => {
  const root = testDir("tt-folder-compiler-");
  const other = testDir("tt-folder-compiler-other-");
  const absolute = path.join(other, "bin", "ttc");
  assert.equal(folderCompiler(path.join("tools", "ttc"), root), path.join(root, "tools", "ttc"));
  assert.equal(folderCompiler(`.${path.sep}ttc`, root), path.join(root, "ttc"));
  assert.equal(folderCompiler(absolute, root), absolute);
  assert.equal(folderCompiler("ttc", root), "ttc", "a bare command name is looked up on PATH");
  assert.equal(folderCompiler("  ", root), findCompiler("", [root]));

  const build = path.join(other, "target", "debug", "ttc");
  fs.mkdirSync(path.dirname(build), { recursive: true });
  fs.writeFileSync(build, "");
  assert.notEqual(folderCompiler("", root), build, "another folder's build is not this folder's compiler");
  assert.equal(folderCompiler("", other), build);
});
