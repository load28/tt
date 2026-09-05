/* The workspace folders the server resolves things against.
 *
 * A root is where the compiler is looked for (`findCompiler`), where the
 * TypeScript toolchain is looked for (`findTsgo`), and what a relative
 * `tt.sidecarDir` is relative to. The client can add and remove folders
 * while the server runs, so these are not fixed at startup. */
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
