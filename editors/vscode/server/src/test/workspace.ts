/* Where a test's project lives.
 *
 * A `.tt` project resolves its TypeScript from `node_modules` walking
 * upwards (`src/typescript/toolchain.rs`), so a project in the system temp
 * directory has none and every typed answer comes back empty — which reads
 * as "the feature is broken" rather than "this project has no TypeScript".
 * Rooting the case under the repository gives it the repository's install,
 * exactly as a package of a monorepo inherits its root's (TASK-256). */
import * as fs from "node:fs";
import * as path from "node:path";

/** The repository root — five levels above `server/out/test`. */
const REPO_ROOT = path.resolve(__dirname, "..", "..", "..", "..", "..");

/** A fresh directory for one case, under the repository's build tree. */
export function caseDir(prefix: string): string {
  const base = path.join(REPO_ROOT, "target", "tt-tests");
  fs.mkdirSync(base, { recursive: true });
  return fs.mkdtempSync(path.join(base, prefix));
}
