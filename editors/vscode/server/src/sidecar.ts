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
import { createHash } from "node:crypto";
import * as fs from "node:fs";
import * as path from "node:path";


/** What to do when an `.tt` file is saved. */
export type SidecarMode = "off" | "refresh" | "always";

/** Outcome of one refresh, for logging and tests. */
export type SidecarResult =
  | { kind: "written"; files: string[] }
  | { kind: "skipped"; reason: string }
  | { kind: "failed"; detail: string };

export class WriteLedger {
  private readonly entries = new Map<
    string,
    { generation: number; fingerprint: string | null }
  >();
  private generation = 0;

  expect(files: string[]): number {
    this.generation += 1;
    for (const file of files) {
      this.entries.set(file, { generation: this.generation, fingerprint: null });
    }
    return this.generation;
  }

  settle(generation: number, written: boolean): void {
    for (const [file, entry] of this.entries) {
      if (entry.generation !== generation) continue;
      const fingerprint = written ? fingerprintOf(file) : null;
      if (fingerprint === null) this.entries.delete(file);
      else entry.fingerprint = fingerprint;
    }
  }

  owns(file: string): boolean {
    const entry = this.entries.get(file);
    if (entry === undefined) return false;
    if (entry.fingerprint === null) return true;
    if (fingerprintOf(file) === entry.fingerprint) return true;
    this.entries.delete(file);
    return false;
  }
}

export const selfWrites = new WriteLedger();

function fingerprintOf(file: string): string | null {
  try {
    return createHash("sha1").update(fs.readFileSync(file)).digest("hex");
  } catch {
    return null;
  }
}

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
  root?: string,
): Promise<SidecarResult> {
  if (mode === "off") return { kind: "skipped", reason: "disabled" };
  if (!ttPath.endsWith(".tt") && !ttPath.endsWith(".ttx")) {
    return { kind: "skipped", reason: "not a tt source" };
  }

  const existing =
    outDir === undefined
      ? [`${ttPath}.d.ts`].filter(exists)
      : treeSidecarsOf(ttPath, outDir, root);
  if (mode === "refresh" && existing.length === 0) {
    return { kind: "skipped", reason: "no sidecar to refresh" };
  }
  const targets =
    existing.length > 0
      ? existing
      : [
          outDir === undefined
            ? `${ttPath}.d.ts`
            : mirroredDeclaration(ttPath, outDir, mirrorBase(ttPath, root)),
        ];

  const written: string[] = [];
  for (const target of targets) {
    const result = await writeSidecar(compiler, ttPath, target);
    if (result.kind !== "written") return result;
    written.push(...result.files);
  }
  return { kind: "written", files: written };
}

async function writeSidecar(
  compiler: string,
  ttPath: string,
  declarationTarget: string,
): Promise<SidecarResult> {
  const args = ["--types", ttPath, "-o", path.dirname(declarationTarget)];
  const files = [declarationTarget, `${declarationTarget}.map`];
  const generation = selfWrites.expect(files);
  const result = await run(compiler, args, files);
  selfWrites.settle(generation, result.kind === "written");
  return result;
}

function mirrorBase(ttPath: string, root: string | undefined): string {
  const dir = path.dirname(path.resolve(ttPath));
  return root !== undefined && isWithin(path.resolve(root), dir)
    ? path.resolve(root)
    : dir;
}

function isWithin(ancestor: string, dir: string): boolean {
  const relative = path.relative(ancestor, dir);
  return relative === "" || (!relative.startsWith("..") && !path.isAbsolute(relative));
}

function mirroredDeclaration(ttPath: string, outDir: string, base: string): string {
  return `${path.join(path.resolve(outDir), path.relative(base, path.resolve(ttPath)))}.d.ts`;
}

function treeSidecarsOf(ttPath: string, outDir: string, root: string | undefined): string[] {
  const source = path.resolve(ttPath);
  const top = mirrorBase(ttPath, root);
  const bases: string[] = [];
  for (let dir = path.dirname(source); ; dir = path.dirname(dir)) {
    bases.push(dir);
    if (dir === top || path.dirname(dir) === dir) break;
  }
  return bases
    .map((base) => mirroredDeclaration(source, outDir, base))
    .filter((declaration) => exists(declaration) && mapNamesSource(`${declaration}.map`, source));
}

function mapNamesSource(mapFile: string, source: string): boolean {
  let parsed: { sources?: unknown; sourceRoot?: unknown };
  try {
    parsed = JSON.parse(fs.readFileSync(mapFile, "utf8")) as typeof parsed;
  } catch {
    return false;
  }
  if (!Array.isArray(parsed.sources)) return false;
  const sourceRoot = typeof parsed.sourceRoot === "string" ? parsed.sourceRoot : "";
  const wanted = canonical(source);
  return parsed.sources.some(
    (entry) =>
      typeof entry === "string" &&
      canonical(path.resolve(path.dirname(mapFile), sourceRoot, entry)) === wanted,
  );
}

function canonical(file: string): string {
  try {
    return fs.realpathSync(file);
  } catch {
    return path.resolve(file);
  }
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
