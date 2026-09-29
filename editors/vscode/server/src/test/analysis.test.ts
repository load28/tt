/* Unit tests for the text-shape utilities — run with `node --test` after
 * compiling (npm test). tt *semantics* (visible variants, match sites) and
 * the cursor's syntactic context are the compiler's answers via the
 * server's `declarations` and `ttCompletions` methods and are covered by
 * server.test.ts and the engine's own tests; what is pinned here is the
 * word at the cursor. */
import * as assert from "node:assert/strict";
import { test } from "node:test";

import { isIdent, wordAt } from "../analysis";

test("wordAt covers the identifier under the cursor", () => {
  const src = "match (value)";
  const w = wordAt(src, 2)!;
  assert.equal(w.word, "match");
  assert.equal(w.start, 0);
  // At the word's end, still the word.
  assert.equal(wordAt(src, 5)!.word, "match");
  assert.equal(wordAt(src, 6), null);
});

test("isIdent rejects reserved words and bad starts", () => {
  assert.ok(isIdent("Shape"));
  assert.ok(!isIdent("variant"));
  assert.ok(!isIdent("1x"));
});
