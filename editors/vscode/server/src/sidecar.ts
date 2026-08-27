/* --------------------------------------------------------------------------
 * On-save regeneration of the editor sidecars (`x.tt.d.ts` + `.map`).
 *
 * A `.ts` file importing `"./x.tt"` only type-checks when a declaration file
 * sits next to the module, and "go to definition" only lands in the original
 * when that declaration carries a map whose `sources` is the `.tt` file
 * (see `ttc --help`, `--sidecar`).
 *
 * Both come from one `ttc` run: the compiler lowers the `.tt` files, hands
 * them to the real TypeScript project the workspace is configured with, and
 * writes what that project emits — declarations from the compiler, map from
 * ttc. Nothing here knows about TypeScript's API, so nothing here breaks
 * when that API changes.
 * ----------------------------------------------------------------------- */
import { execFile } from "node:child_process";
import * as fs from "node:fs";
import * as path from "node:path";


/** What to do when an `.tt` file is saved. */
export type SidecarMode = "off" | "refresh" | "always";

/** Outcome of one refresh, for logging and tests. */
export type SidecarResult =
  | { kind: "written"; files: string[] }
  | { kind: "skipped"; reason: string }
  | { kind: "failed"; detail: string };

/**
 * Rebuilds the sidecar for one `.tt` file.
 *
 * In `"refresh"` mode (the default) a sidecar is only rewritten when one is
 * already there — a project opts in by running `ttc --types` once, and
 * nothing appears in a workspace that never asked for it. `"always"` creates
 * it on first save.
 */
export async function refreshSidecar(
  compiler: string,
  ttPath: string,
  mode: SidecarMode,
  outDir?: string,
): Promise<SidecarResult> {
  if (mode === "off") return { kind: "skipped", reason: "disabled" };
  if (!ttPath.endsWith(".tt") && !ttPath.endsWith(".ttx")) {
    return { kind: "skipped", reason: "not a tt source" };
  }

  // Declarations either sit next to the source or in their own tree (which
  // TypeScript merges back with `rootDirs`); the refresh has to look where
  // they actually are.
  const base =
    outDir === undefined ? ttPath : path.join(outDir, path.basename(ttPath));
  const declarationTarget = `${base}.d.ts`;
  if (mode === "refresh" && !exists(declarationTarget)) {
    return { kind: "skipped", reason: "no sidecar to refresh" };
  }

  // `-o` is always passed: on its own `--types` writes to `.tt-types`, the
  // directory a project opts into, and the editor's sidecar goes wherever
  // the settings say — beside the source by default.
  const args = ["--types", ttPath, "-o", outDir ?? path.dirname(ttPath)];
  return run(compiler, args, [declarationTarget, `${base}.d.ts.map`]);
}

function exists(file: string): boolean {
  try {
    return fs.existsSync(file);
  } catch {
    return false;
  }
}

/**
 * One `ttc` run.
 *
 * The exit code carries three answers, and the middle one is the whole
 * point: a saved file mid-edit usually has type errors, and the sidecar is
 * written anyway (a stale one is worse than one built from code that does
 * not check yet).
 *
 * - `0` — checked clean, written.
 * - `1` — something was reported, and written all the same.
 * - `2` — the check could not run (a tt-level error left nothing to
 *   lower), so nothing was written and the last good sidecar stands.
 */
function run(compiler: string, args: string[], files: string[]): Promise<SidecarResult> {
  return new Promise((resolve) => {
    execFile(
      compiler,
      args,
      { timeout: 30000, maxBuffer: 8 * 1024 * 1024 },
      (err, _stdout, stderr) => {
        const code = err === null ? 0 : ((err as { code?: number }).code ?? 1);
        if (code === 0 || code === 1) {
          resolve({ kind: "written", files });
          return;
        }
        resolve({ kind: "failed", detail: stderr.trim() || String(err) });
      },
    );
  });
}
