/* The workspace roots, as the client changes them.
 *
 * They decide where the compiler is found, where the TypeScript toolchain
 * is found, and what a relative `tt.sidecarDir` is relative to — so a
 * folder added to the window has to become one, and a folder removed has
 * to stop being one (TASK-342). */
import * as assert from "node:assert/strict";
import { test } from "node:test";
import { pathToFileURL } from "node:url";
import * as os from "node:os";
import * as path from "node:path";

import { applyFolderChange, folderRoots } from "../roots";

const first = path.join(os.tmpdir(), "tt-roots-first");
const second = path.join(os.tmpdir(), "tt-roots-second");
const folder = (dir: string) => ({ uri: pathToFileURL(dir).toString() });

test("folders arrive as paths, in the order the client sent them", () => {
  assert.deepEqual(folderRoots([folder(first), folder(second)]), [
    first,
    second,
  ]);
  assert.deepEqual(folderRoots(undefined), []);
  assert.deepEqual(folderRoots(null), []);
});

test("a folder the server cannot reach on disk is not a root", () => {
  assert.deepEqual(
    folderRoots([{ uri: "vscode-vfs://github/load28/tt" }, folder(first)]),
    [first],
  );
});

test("the same folder twice is one root", () => {
  assert.deepEqual(folderRoots([folder(first), folder(first)]), [first]);
});

test("an added folder becomes a root and the existing ones keep their order", () => {
  assert.deepEqual(
    applyFolderChange([first], { added: [folder(second)], removed: [] }),
    [first, second],
  );
});

test("a removed folder stops being a root", () => {
  assert.deepEqual(
    applyFolderChange([first, second], { added: [], removed: [folder(first)] }),
    [second],
  );
});

test("a folder that is removed and added again in one event stays a root", () => {
  assert.deepEqual(
    applyFolderChange([first], {
      added: [folder(first)],
      removed: [folder(first)],
    }),
    [first],
  );
});

test("adding a folder that is already a root changes nothing", () => {
  assert.deepEqual(applyFolderChange([first], { added: [folder(first)] }), [
    first,
  ]);
});
