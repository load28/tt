/* Which watched-file changes are news to this server.
 *
 * The client watches the workspace so the server learns about edits made
 * outside it — a file created, deleted, or changed by a build, a branch
 * switch, or another editor. Acting on one is expensive by design: the
 * engine's project graphs and every buffer's projection are rebuilt,
 * because what the project contains may have changed.
 *
 * The user's own Ctrl+S arrives through the same channel and is not news.
 * The server already holds that buffer, and its text reached the engine as
 * it was typed, so the state a save would rebuild is the state that is
 * already there. Treating a save as an external edit makes every save pay a
 * cold start and re-raises standing notices the user has already read.
 *
 * Creation and deletion still count for an open document: those change what
 * the project contains, whoever is holding the file.
 */

/** The `FileChangeType` values the protocol defines. */
export const CREATED = 1;
export const CHANGED = 2;
export const DELETED = 3;

export interface WatchedChange {
  /** Filesystem path of the changed file. */
  path: string;
  /** The protocol's change type. */
  type: number;
}

/**
 * Whether `change` tells the server something it does not already have.
 *
 * `openPaths` are the buffers the server holds, by filesystem path.
 */
export function isExternalChange(change: WatchedChange, openPaths: ReadonlySet<string>): boolean {
  if (change.type === CHANGED && openPaths.has(change.path)) return false;
  return true;
}
