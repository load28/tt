import assert from "node:assert/strict";
import path from "node:path";
import test from "node:test";

import { containingRoot } from "../roots";

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
