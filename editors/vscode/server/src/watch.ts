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
 * A write this server made itself (a sidecar rebuilt on save) is not news
 * either, for as long as the disk still holds what it wrote.
 */
import * as fs from "node:fs";

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

export interface WriteOwnership {
  owns(path: string): boolean;
}

/**
 * Whether `change` tells the server something it does not already have.
 *
 * `openBuffers` are the buffers the server holds, by filesystem path;
 * `ownWrites` answers for the files the server wrote itself.
 */
export function isExternalChange(
  change: WatchedChange,
  openBuffers: ReadonlyMap<string, string>,
  ownWrites?: WriteOwnership,
): boolean {
  if (change.type === DELETED) return true;
  if (change.type === CHANGED) {
    const heldText = openBuffers.get(change.path);
    if (heldText !== undefined) {
      try {
        // A save publishes the text already held by the server. A write by
        // another process may change the disk without changing that buffer.
        if (fs.readFileSync(change.path, "utf8") === heldText) return false;
      } catch {
        // The path disappeared between the watcher event and this read.
        return true;
      }
    }
  }
  if (ownWrites?.owns(change.path)) return false;
  return true;
}
