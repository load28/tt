import * as assert from "node:assert/strict";
import * as path from "node:path";
import { test } from "node:test";

import { isWithin } from "../paths";

test("a directory whose name begins with two dots is within its parent", () => {
  const root = path.resolve("/workspace/project");
  assert.equal(isWithin(root, path.join(root, "..cache")), true);
  assert.equal(isWithin(root, path.join(root, "..cache", "src")), true);
  assert.equal(isWithin(root, path.join(root, "...")), true);
});

test("only a real parent segment leaves the ancestor", () => {
  const root = path.resolve("/workspace/project");
  assert.equal(isWithin(root, root), true);
  assert.equal(isWithin(root, path.dirname(root)), false);
  assert.equal(isWithin(root, path.join(path.dirname(root), "other")), false);
  assert.equal(isWithin(root, path.resolve("/elsewhere")), false);
});
