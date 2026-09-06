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

import { applyFolderChange, folderRoots, sidecarLocation } from "../roots";

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

/* `tt.sidecarDir` is a directory relative to the workspace root. An empty
 * setting and a setting with no root to resolve against are different
 * answers — the first asks for sidecars beside the source, the second names
 * a place the server cannot compute — and collapsing them wrote generated
 * declarations into a tree the user had asked to keep clean, silently
 * (TASK-344). */
test("an empty setting asks for sidecars beside the source", () => {
  assert.deepEqual(sidecarLocation("", path.join(first, "a.tt"), [first]), {
    kind: "adjacent",
  });
  assert.deepEqual(sidecarLocation("   ", path.join(first, "a.tt"), [first]), {
    kind: "adjacent",
  });
});

test("a relative setting resolves against the folder the file is in", () => {
  assert.deepEqual(
    sidecarLocation(".tt-types", path.join(second, "a.tt"), [first, second]),
    { kind: "directory", path: path.join(second, ".tt-types") },
  );
});

test("nested folders resolve against the deepest one that contains the file", () => {
  const inner = path.join(first, "packages", "app");
  assert.deepEqual(
    sidecarLocation(".tt-types", path.join(inner, "a.tt"), [first, inner]),
    { kind: "directory", path: path.join(inner, ".tt-types") },
  );
});

test("an absolute setting needs no root", () => {
  assert.deepEqual(sidecarLocation(second, path.join(first, "a.tt"), []), {
    kind: "directory",
    path: second,
  });
});

test("a relative setting with no containing folder is unresolved, not adjacent", () => {
  assert.deepEqual(
    sidecarLocation(".tt-types", path.join(second, "loose.tt"), [first]),
    { kind: "unresolved", configured: ".tt-types" },
  );
  assert.deepEqual(sidecarLocation(".tt-types", path.join(second, "a.tt"), []), {
    kind: "unresolved",
    configured: ".tt-types",
  });
});
