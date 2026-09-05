/* The one-shot fallback's reading of `ttc --check` output.
 *
 * The fallback exists so that a compiler without `--server` — or one the
 * engine has given up on — still reports the same errors. It only does that
 * if it can read what the compiler prints, so the shape is pinned here
 * against the compiler's own rendered form (TASK-335). */
import * as assert from "node:assert/strict";
import { test } from "node:test";

import { parseStderr } from "../ttc";

const RENDERED = [
  'error[match-not-exhaustive]: match on variant Shape is not exhaustive: missing "Square"',
  " --> /tmp/x/main.tt:2:35",
  "  |",
  "2 | export const area = (s: Shape) => match (s) {",
  "  |                                   ^^^^^^^^^",
  "  |",
  "  = help: add the missing arms: `Square(side) => undefined,`",
  "",
].join("\n");

test("a rendered diagnostic carries its position, message and rule", () => {
  assert.deepEqual(parseStderr(RENDERED, "/tmp/x/main.tt"), [
    {
      line: 2,
      col: 35,
      message: 'match on variant Shape is not exhaustive: missing "Square"',
      code: "match-not-exhaustive",
    },
  ]);
});

test("a wider line number keeps its location line aligned", () => {
  const wide = [
    "error[val-mutation]: cannot mutate through val binding `state`",
    "  --> /tmp/x/main.tt:10:3",
    "   |",
    "10 |   state.count = 1;",
    "   |   ^^^^^",
  ].join("\n");
  assert.deepEqual(parseStderr(wide, "/tmp/x/main.tt"), [
    {
      line: 10,
      col: 3,
      message: "cannot mutate through val binding `state`",
      code: "val-mutation",
    },
  ]);
});

test("every diagnostic in a run is read, and other files are left alone", () => {
  const both = [
    RENDERED,
    "error[unknown-case]: variant Shape has no case `Circel`",
    " --> /tmp/x/other.tt:13:30",
    "  |",
    "error[val-mutation]: cannot mutate through val binding `state`",
    " --> /tmp/x/main.tt:9:3",
    "  |",
  ].join("\n");
  assert.deepEqual(
    parseStderr(both, "/tmp/x/main.tt").map((d) => [d.line, d.code]),
    [
      [2, "match-not-exhaustive"],
      [9, "val-mutation"],
    ],
  );
});

test("a compiler that predates the rendered form still reports", () => {
  const legacy = [
    "ttc: /tmp/x/main.tt:2:35: match on variant Shape is not exhaustive",
    "ttc: /tmp/x/main.tt: output verification failed",
    "ttc: /tmp/x/other.tt:1:1: not this file",
  ].join("\n");
  assert.deepEqual(parseStderr(legacy, "/tmp/x/main.tt"), [
    { line: 2, col: 35, message: "match on variant Shape is not exhaustive" },
    { line: 0, col: 0, message: "output verification failed" },
  ]);
});

test("usage errors and progress lines are not diagnostics", () => {
  const noise = [
    "ttc: no such file or directory: nope.tt",
    "ttc: /tmp/x/main.tt → /tmp/out/main.ts",
    "error: something without a location",
    "warning[a-rule]: a warning about a file that is not ours",
    " --> /tmp/x/other.tt:1:1",
  ].join("\n");
  assert.deepEqual(parseStderr(noise, "/tmp/x/main.tt"), []);
});

test("a warning is a diagnostic too", () => {
  const warning = [
    "warning[some-rule]: a thing worth knowing",
    " --> /tmp/x/main.tt:4:7",
  ].join("\n");
  assert.deepEqual(parseStderr(warning, "/tmp/x/main.tt"), [
    { line: 4, col: 7, message: "a thing worth knowing", code: "some-rule" },
  ]);
});
