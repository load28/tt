/* The workspace folders the server resolves things against.
 *
 * A root is where the compiler is looked for (`findCompiler`), where the
 * TypeScript toolchain is looked for (`findTsgo`), and what a relative
 * `tt.sidecarDir` is relative to. The client can add and remove folders
 * while the server runs, so these are not fixed at startup. */
import * as path from "node:path";

import { URI } from "vscode-uri";

/** A workspace folder as the protocol carries it. */
export interface FolderLike {
  uri: string;
}

/** The file-system paths of workspace folders, in the client's order.
 *
 * A folder the server cannot reach through the filesystem is not a root:
 * every use of one is a path lookup on disk. */
export function folderRoots(
  folders: readonly FolderLike[] | null | undefined,
): string[] {
  const roots: string[] = [];
  for (const folder of folders ?? []) {
    const uri = URI.parse(folder.uri);
    if (uri.scheme !== "file") continue;
    if (!roots.includes(uri.fsPath)) roots.push(uri.fsPath);
  }
  return roots;
}

/** The roots after the client reported a change.
 *
 * Removals are applied before additions, so a folder that was removed and
 * added again in one event stays a root, and the folders that were already
 * there keep their order. */
export function applyFolderChange(
  roots: readonly string[],
  event: {
    added?: readonly FolderLike[] | null;
    removed?: readonly FolderLike[] | null;
  },
): string[] {
  const removed = new Set(folderRoots(event.removed));
  const kept = roots.filter((root) => !removed.has(root));
  for (const added of folderRoots(event.added)) {
    if (!kept.includes(added)) kept.push(added);
  }
  return kept;
}

/** Where one file's sidecar belongs.
 *
 * `tt.sidecarDir` is documented as a directory relative to the workspace
 * root, so a file that belongs to no root gives a relative setting no base
 * to resolve against. That is `unresolved`, and it is not the same answer
 * as `adjacent`: the latter is what an empty setting asks for, the former
 * is a configured location the server cannot compute. Collapsing the two
 * writes declarations somewhere the user did not ask for and says nothing
 * about it, so they are separate cases here and the caller decides. */
export type SidecarLocation =
  | { kind: "adjacent" }
  | { kind: "directory"; path: string }
  | { kind: "unresolved"; configured: string };

/** The longest root that contains `filePath`, or `undefined`.
 *
 * Nested roots both match, and the deepest one is the folder the file
 * actually belongs to. */
export function containingRoot(
  roots: readonly string[],
  filePath: string,
): string | undefined {
  return roots
    .filter((candidate) => filePath.startsWith(`${candidate}${path.sep}`))
    .sort((a, b) => b.length - a.length)[0];
}

export function sidecarLocation(
  configured: string,
  filePath: string,
  roots: readonly string[],
): SidecarLocation {
  const dir = configured.trim();
  if (dir === "") return { kind: "adjacent" };
  if (path.isAbsolute(dir)) return { kind: "directory", path: dir };

  const root = containingRoot(roots, filePath);
  return root === undefined
    ? { kind: "unresolved", configured: dir }
    : { kind: "directory", path: path.join(root, dir) };
}
