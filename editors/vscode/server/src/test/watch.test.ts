import assert from "node:assert/strict";
import test from "node:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";

import { CHANGED, CREATED, DELETED, isExternalChange } from "../watch";

const open = new Map([["/w/open.tt", "source text"]]);

test("a save of a held buffer is not news", () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "tt-open-save-"));
  try {
    const file = path.join(dir, "open.tt");
    fs.writeFileSync(file, "source text");
    const held = new Map([[file, "source text"]]);
    assert.equal(isExternalChange({ path: file, type: CHANGED }, held), false);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

test("an edit to a file nobody holds is news", () => {
  assert.equal(isExternalChange({ path: "/w/other.tt", type: CHANGED }, open), true);
});

test("creating or deleting a held buffer is still news", () => {
  assert.equal(isExternalChange({ path: "/w/open.tt", type: CREATED }, open), true);
  assert.equal(isExternalChange({ path: "/w/open.tt", type: DELETED }, open), true);
});

test("with nothing open every change is news", () => {
  const none = new Map<string, string>();
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

test("an external write to an open buffer is news when disk content differs", () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "tt-open-watch-"));
  try {
    const file = path.join(dir, "open.tt");
    fs.writeFileSync(file, "export const value = 1;\n");
    const held = new Map([[file, "export const value = 1;\n"]]);
    assert.equal(isExternalChange({ path: file, type: CHANGED }, held), false);
    fs.writeFileSync(file, "export const value = 2;\n");
    assert.equal(isExternalChange({ path: file, type: CHANGED }, held), true);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});
