/* Unit tests for the text-shape utilities — run with `node --test` after
 * compiling (npm test). tt *semantics* (visible variants, match sites) are
 * the compiler's answer via the server's `declarations` method and are
 * covered by server.test.ts; what is pinned here is masking and cursor
 * context. */
import * as assert from "node:assert/strict";
import { test } from "node:test";

import { isIdent, maskNonCode, memberAccessAt, wordAt } from "../analysis";

test("maskNonCode blanks strings, comments and template text", () => {
  const src = 'const s = "variant A {}"; // variant B {}\nconst t = `match (${x}) {`;';
  const masked = maskNonCode(src);
  assert.equal(masked.length, src.length);
  assert.ok(!masked.includes("variant A"));
  assert.ok(!masked.includes("variant B"));
  // Code inside a template interpolation is kept.
  assert.ok(masked.includes("x"));
});

test("maskNonCode blanks regex literals but not division", () => {
  const src = "const r = /variant X/; const q = a / b;";
  const masked = maskNonCode(src);
  assert.ok(!masked.includes("variant X"));
  assert.ok(masked.includes("a / b"));
});

test("memberAccessAt finds the base identifier before a dot", () => {
  const src = "const v = Shape.";
  assert.equal(memberAccessAt(src, src.length), "Shape");
  assert.equal(memberAccessAt(src, 5), null);
});

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

test("a JSX closing tag is not a regex in a .ttx buffer", () => {
  // `<` is a position a regex may follow, so without the JSX surface the
  // slash of `</p>` starts an imagined literal that swallows the rest of
  // the line — and the cursor context after it is whatever that left.
  const src = "const el = <><p>{t.a}</p><p>{t.</p></>;";
  assert.equal(maskNonCode(src, true), src);
  assert.equal(memberAccessAt(maskNonCode(src, true), src.lastIndexOf("{t.") + 3), "t");

  // A comparison against a regex is still a regex, in either surface.
  const compared = "const b = a < /x[/]y/g;";
  assert.equal(maskNonCode(compared, true), "const b = a <         ;");
  assert.equal(maskNonCode(compared, false), "const b = a <         ;");
});
