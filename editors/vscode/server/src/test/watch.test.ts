import assert from "node:assert/strict";
import test from "node:test";

import { CHANGED, CREATED, DELETED, isExternalChange } from "../watch";

const open = new Set(["/w/open.tt"]);

test("a save of a held buffer is not news", () => {
  assert.equal(isExternalChange({ path: "/w/open.tt", type: CHANGED }, open), false);
});

test("an edit to a file nobody holds is news", () => {
  assert.equal(isExternalChange({ path: "/w/other.tt", type: CHANGED }, open), true);
});

test("creating or deleting a held buffer is still news", () => {
  assert.equal(isExternalChange({ path: "/w/open.tt", type: CREATED }, open), true);
  assert.equal(isExternalChange({ path: "/w/open.tt", type: DELETED }, open), true);
});

test("with nothing open every change is news", () => {
  const none = new Set<string>();
  for (const type of [CREATED, CHANGED, DELETED]) {
    assert.equal(isExternalChange({ path: "/w/open.tt", type }, none), true);
  }
});

test("a file this server wrote itself is not news while the disk still holds it", () => {
  const written = new Set(["/w/open.tt.d.ts"]);
  const ownWrites = { owns: (path: string) => written.has(path) };
  assert.equal(isExternalChange({ path: "/w/open.tt.d.ts", type: CHANGED }, open, ownWrites), false);
  assert.equal(isExternalChange({ path: "/w/open.tt.d.ts", type: CREATED }, open, ownWrites), false);
  assert.equal(isExternalChange({ path: "/w/open.tt.d.ts", type: DELETED }, open, ownWrites), true);
  assert.equal(isExternalChange({ path: "/w/other.tt.d.ts", type: CHANGED }, open, ownWrites), true);
  written.clear();
  assert.equal(isExternalChange({ path: "/w/open.tt.d.ts", type: CHANGED }, open, ownWrites), true);
});
