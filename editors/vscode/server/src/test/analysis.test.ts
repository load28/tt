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

test("an apostrophe in JSX text does not start a string", () => {
  const src = "const el = <p>Don't {user.}</p>;";
  const masked = maskNonCode(src, true);
  assert.equal(masked.length, src.length);
  assert.ok(!masked.includes("Don't"));
  assert.equal(memberAccessAt(masked, src.indexOf("user.") + 5), "user");
  assert.equal(maskNonCode("const el = <p>Don't {user.}</p>;", false).includes("user"), false);
});

test("quotes in JSX attributes are attribute text, not code strings", () => {
  const src = `const el = <a title="it's" href='say "hi"' onClick={() => go(x.)}>{y.}</a>;`;
  const masked = maskNonCode(src, true);
  assert.equal(masked.length, src.length);
  assert.ok(!masked.includes("it's"));
  assert.ok(!masked.includes('"hi"'));
  assert.equal(memberAccessAt(masked, src.indexOf("x.") + 2), "x");
  assert.equal(memberAccessAt(masked, src.indexOf("y.") + 2), "y");
});

test("nested elements and expression containers return to code", () => {
  const src = [
    "const list = (",
    "  <ul>",
    "    {items.map(item => <li key={item.id}>{item.name} isn't {other.}</li>)}",
    "  </ul>",
    ");",
    "const after = a / b;",
  ].join("\n");
  const masked = maskNonCode(src, true);
  assert.equal(masked.length, src.length);
  assert.ok(!masked.includes("isn't"));
  assert.ok(masked.includes("item.id"));
  assert.ok(masked.includes("item.name"));
  assert.equal(memberAccessAt(masked, src.indexOf("other.") + 6), "other");
  assert.ok(masked.includes("a / b"));
  assert.equal(memberAccessAt(masked, src.indexOf("item.name") + 5), "item");
});

test("a generic arrow function in a .ttx buffer is code, not an element", () => {
  const src = [
    "const id = <T,>(x: T) => x;",
    "const pick = <T extends object>(x: T) => x;",
    "const keep = <const T,>(x: T) => x;",
    "const v = q.",
  ].join("\n");
  const masked = maskNonCode(src, true);
  assert.equal(masked, src);
  assert.equal(memberAccessAt(masked, src.length), "q");
});

test("a fragment and a self-closing element are elements", () => {
  const src = "const el = <><img src='a.png' />{'literal'}{v.}</>;";
  const masked = maskNonCode(src, true);
  assert.equal(masked.length, src.length);
  assert.ok(!masked.includes("a.png"));
  assert.ok(!masked.includes("literal"));
  assert.equal(memberAccessAt(masked, src.indexOf("v.") + 2), "v");
});
