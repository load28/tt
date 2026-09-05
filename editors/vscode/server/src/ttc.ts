/* --------------------------------------------------------------------------
 * Diagnostics come from the real compiler: through the engine session
 * (`ttc --server`, see engine.ts) when one is available, and through the
 * one-shot commands otherwise — the two produce the same diagnostics, so
 * the fallback is invisible. Editor errors are byte-for-byte the compiler's
 * own (`file:line:col: message`).
 * ----------------------------------------------------------------------- */
import { execFile } from "child_process";
import * as crypto from "crypto";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";

import { packageCompiler } from "./install";
import { engineRequest } from "./engine";

/** A typed check opens a project and starts a compiler; it is slower than
 * `--check` by that much, and a project that will not open must not hold the
 * editor forever. */
const TYPED_CHECK_TIMEOUT_MS = 30000;

export interface TtcDiagnostic {
  /** 1-based; 0 means "no position" (output-verification errors). */
  line: number;
  /** 1-based; 0 means "no position". */
  col: number;
  /** End of the range the diagnostic covers, past its last character;
   * 1-based, 0 (or absent) when the compiler reported a position only —
   * the squiggle then falls back to the word at the position. The compiler
   * sends this for a construct it knows the extent of, so the underline
   * covers `try parse(text)` or `match (shape)` as a whole. */
  endLine?: number;
  /** See {@link TtcDiagnostic.endLine}. */
  endCol?: number;
  message: string;
  /** Stable tt rule identity; older/one-shot compilers may omit it. */
  code?: string;
  /** How to fix it, as the compiler reports it. An entry with an `edit` is
   * a fix the editor can apply; one without names advice only. Older
   * compilers omit the field. */
  suggestions?: TtcSuggestion[];
  /** Secondary labeled spans ("the piped value is produced here"), absent
   * when the diagnostic has only its primary range or the compiler
   * predates them. */
  labels?: TtcLabel[];
}

/** One secondary labeled span of a diagnostic — 1-based line/column pairs,
 * `endLine`/`endCol` past the last character, the same coordinates the
 * diagnostic itself is reported in. `path` names another file; without it
 * the span is in the diagnostic's own file. */
export interface TtcLabel {
  line: number;
  col: number;
  endLine: number;
  endCol: number;
  message: string;
  path?: string;
}

/** One way to resolve a diagnostic (`ttc`'s `Suggestion`). */
export interface TtcSuggestion {
  message: string;
  /** The replacement, when the compiler can name one. 1-based line/column,
   * `endLine`/`endCol` past the last replaced character — the same
   * coordinates the diagnostic itself is reported in. */
  edit?: {
    line: number;
    col: number;
    endLine: number;
    endCol: number;
    replacement: string;
  } | null;
}

/** Why a compiler could not be run: it is not there, or it is there and
 * the operating system refused to start it — an unset execute bit, a path
 * that names a directory, a file that is not an executable. Both leave the
 * editor without diagnostics, and both are fixed by the user, so both are
 * worth saying out loud. */
export type UnusableCompiler = "missing" | "not-executable";

export type TtcResult =
  | { kind: "ok"; diagnostics: TtcDiagnostic[] }
  | { kind: "not-found"; compiler: string; reason: UnusableCompiler }
  | { kind: "failed"; detail: string };

let tmpDir: string | null = null;
function tempDir(): string {
  if (tmpDir === null) {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "tt-lsp-"));
  }
  return tmpDir;
}

const CANDIDATE_PATHS = [
  path.join("target", "release", "ttc"),
  path.join("target", "debug", "ttc"),
  path.join("target", "release", "ttc.exe"),
  path.join("target", "debug", "ttc.exe"),
];

/** The newest compiler build in a development workspace. `cargo build` and
 * `cargo build --release` are both legitimate; fixed profile precedence can
 * select an older protocol whenever both artifacts happen to exist. */
function workspaceCompiler(root: string): string {
  let selected = "";
  let selectedMtime = -Infinity;
  for (const rel of CANDIDATE_PATHS) {
    const candidate = path.join(root, rel);
    try {
      const mtime = fs.statSync(candidate).mtimeMs;
      if (mtime > selectedMtime) {
        selected = candidate;
        selectedMtime = mtime;
      }
    } catch {
      // Missing and unreadable candidates do not participate.
    }
  }
  return selected;
}

/**
 * Resolve the compiler to run:
 *
 * 1. the explicit `tt.compilerPath` setting;
 * 2. a binary built in a workspace root — the TT repository itself, open in
 *    the editor that is developing it;
 * 3. the ttc of the `@openload28/tt-lang` package the project installed, which the
 *    package resolves itself (install.ts) — a published install, a `file:`
 *    development install, or `TTC_BINARY`;
 * 4. `ttc` on PATH.
 *
 * Step 3 is how a project that merely *consumes* tt gets a compiler: the one
 * it installed, the same one `npx ttc` runs. Everything above it is for
 * someone building the compiler; everything below it is a machine-wide
 * install.
 */
export function findCompiler(
  configuredPath: string,
  workspaceRoots: string[],
): string {
  if (configuredPath.trim() !== "") return configuredPath.trim();
  for (const root of workspaceRoots) {
    const built = workspaceCompiler(root);
    if (built !== "") return built;
  }
  const installed = packageCompiler(workspaceRoots);
  if (installed !== "") return installed;
  return "ttc";
}

/**
 * Whether a TypeScript language toolchain is around, from the one place
 * the engine looks: the TypeScript the project installed
 * (`src/typescript/toolchain.rs`). The engine resolves its own; this mirror
 * exists so the tests can tell "the feature answered nothing" from "the
 * toolchain is missing" — and skip, not fail, on the latter.
 *
 * A guard that mirrors only part of the compiler's rules answers "no
 * toolchain" where the compiler finds one, and then a suite skips, or an
 * unguarded case fails, depending on which half of the rules the machine
 * happens to satisfy (TASK-217).
 */
export function findTsgo(workspaceRoots: string[]): string {
  const platform = `${process.platform}-${process.arch}`;
  const suffix = process.platform === "win32" ? ".exe" : "";
  // The published layout: the executable sits in the platform package's
  // `lib/`, named for its distribution — `typescript` ships `tsc`, the
  // preview channel ships `tsgo`.
  const packages: [string, string][] = [
    [`@typescript/typescript-${platform}`, "tsc"],
    [`@typescript/native-preview-${platform}`, "tsgo"],
  ];
  for (const start of [...workspaceRoots, process.cwd()]) {
    let dir = start;
    for (;;) {
      for (const [pkg, bin] of packages) {
        const exe = path.join(dir, "node_modules", pkg, "lib", `${bin}${suffix}`);
        if (exists(exe)) return exe;
      }
      const parent = path.dirname(dir);
      if (parent === dir) break;
      dir = parent;
    }
  }
  return "";
}

function exists(file: string): boolean {
  try {
    return fs.existsSync(file);
  } catch {
    return false;
  }
}

/** Run `ttc --check` on the buffer contents and parse stderr diagnostics. */
export async function runCheck(
  compiler: string,
  text: string,
  docName: string,
  verify: boolean,
): Promise<TtcResult> {
  // The engine server answers with the same diagnostics and no process
  // spawn; the one-shot below is the fallback and the reference.
  const answer = await engineRequest(
    compiler,
    "check",
    { text, filename: path.basename(docName), verify },
    15000,
  );
  if (answer && "result" in answer) {
    const result = answer.result as { diagnostics?: TtcDiagnostic[] };
    return {
      kind: "ok",
      diagnostics: (result.diagnostics ?? []).map((d) => ({
        line: d.line,
        col: d.col,
        endLine: d.endLine,
        endCol: d.endCol,
        message: d.message,
        code: d.code,
        suggestions: d.suggestions,
      })),
    };
  }
  return runCheckOnce(compiler, text, docName, verify);
}

/** The one-shot `ttc --check`, via a temp file. */
function runCheckOnce(
  compiler: string,
  text: string,
  docName: string,
  verify: boolean,
): Promise<TtcResult> {
  const rawBase = path.basename(docName).replace(/[^\w.-]/g, "_") || "buffer";
  const base = rawBase.endsWith(".tt") || rawBase.endsWith(".ttx")
    ? rawBase
    : `${rawBase}.tt`;
  const hash = crypto.createHash("sha1").update(docName).digest("hex");
  const file = path.join(tempDir(), `${hash.slice(0, 8)}-${base}`);

  try {
    fs.writeFileSync(file, text);
  } catch (e) {
    return Promise.resolve({ kind: "failed", detail: String(e) });
  }

  const args = ["--check"];
  if (!verify) args.push("--no-verify");
  args.push(file);

  return new Promise((resolve) => {
    execFile(
      compiler,
      args,
      { timeout: 15000, maxBuffer: 4 * 1024 * 1024 },
      (err, _stdout, stderr) => {
        const unusable = unusableCompiler(err);
        if (unusable) {
          resolve({ kind: "not-found", compiler, reason: unusable });
          return;
        }
        const diagnostics = parseStderr(String(stderr), file);
        if (err && diagnostics.length === 0) {
          // Crashed or timed out without a parseable diagnostic.
          resolve({
            kind: "failed",
            detail: `${compiler} exited abnormally: ${String(stderr).trim() || err.message}`,
          });
          return;
        }
        resolve({ kind: "ok", diagnostics });
      },
    );
  });
}

/* ----------------------------------------------------------------------
 * Symbol interface (`ttc --symbols`): the compiler
 * reports a file's tt variant declarations (with 1-based positions) and its
 * direct relative `.tt` imports, including each referenced file's exported
 * declarations — the server consumes this for cross-file features instead
 * of re-implementing import resolution.
 * -------------------------------------------------------------------- */

export interface SymbolsField {
  name: string;
  optional: boolean;
  type: string;
}

export interface SymbolsCase {
  tag: string;
  line: number;
  col: number;
  /** null for a unit case without parens. */
  fields: SymbolsField[] | null;
}

export interface SymbolsVariant {
  name: string;
  exported: boolean;
  generics: string;
  line: number;
  col: number;
  cases: SymbolsCase[];
}

export type SymbolsNames =
  | { kind: "namespace"; name: string }
  | { kind: "named"; entries: { name: string; alias: string | null }[] }
  | { kind: "none" };

export interface SymbolsImport {
  specifier: string;
  names: SymbolsNames;
  /** Path the specifier resolved to, or null if unreadable. */
  resolved: string | null;
  variants: SymbolsVariant[];
}

export interface SymbolsFile {
  file: string;
  variants: SymbolsVariant[];
  imports: SymbolsImport[];
}

/**
 * Run `ttc --symbols` on a file on disk. Returns null when the compiler is
 * missing, predates `--symbols`, or the output is unparseable — callers
 * degrade to single-file behavior.
 */
export function runSymbols(
  compiler: string,
  file: string,
): Promise<SymbolsFile | null> {
  return new Promise((resolve) => {
    execFile(
      compiler,
      ["--symbols", file],
      { timeout: 15000, maxBuffer: 16 * 1024 * 1024 },
      (err, stdout) => {
        if (err) {
          resolve(null);
          return;
        }
        try {
          const parsed = JSON.parse(String(stdout)) as SymbolsFile[];
          resolve(parsed[0] ?? null);
        } catch {
          resolve(null);
        }
      },
    );
  });
}

/**
 * The typed half of the tt diagnostics, for the buffer as it stands.
 *
 * `ttc --check` answers everything tt can decide from the text alone. What
 * it cannot decide is what a TypeScript *type* says — whether `m.set(...)`
 * calls a built-in mutator through a `val` binding, whether a scrutinee's
 * type still allows a case (`language.md` §10.4). Those answers come from
 * the engine's typed pass over the live project, or — as the fallback —
 * from `ttc --check-types --tt-only --overlay`.
 *
 * Every message is the compiler's own, verbatim: the editor decides nothing
 * about `val`, it relays what ttc said (CLAUDE.md, error layers).
 */
export type ValCheckResult =
  | { kind: "ok"; diagnostics: TtcDiagnostic[] }
  /** The check could not run (no toolchain, crash, or nothing to check).
   * Distinct from "ran and found nothing": the caller keeps what it has. */
  | {
      kind: "unavailable";
      detail: string;
      /** Environmental absence stays quiet; an internal backend failure
       * must be surfaced as a compiler failure by the presentation layer. */
      cause: "availability" | "internal";
    };

export async function runTypedCheck(
  compiler: string,
  text: string,
  fsPath: string,
  includeTypes = false,
): Promise<ValCheckResult> {
  // The overlay stands in for a file of the project, so there has to be one:
  // a buffer that was never saved has no place in the project graph yet.
  if (!exists(fsPath)) {
    return {
      kind: "unavailable",
      detail: "the document is not on disk yet",
      cause: "availability",
    };
  }
  // The engine session keeps the project — and the TypeScript compiler
  // behind it — alive between checks, so this answers in milliseconds
  // where the one-shot pays a project open every time. Same diagnostics
  // either way; the outcome mapping below mirrors the one-shot's exactly.
  const answer = await engineRequest(
    compiler,
    "typedCheck",
    { path: fsPath, text, includeTypes },
    TYPED_CHECK_TIMEOUT_MS,
  );
  if (answer && "error" in answer) {
    // The session ran and the request failed (no toolchain, a backend
    // crash) — what the one-shot reports as "could not run".
    return {
      kind: "unavailable",
      detail: answer.error,
      cause: "internal",
    };
  }
  if (answer && "result" in answer) {
    const result = answer.result as {
      blocked?: boolean;
      backendError?: {
        kind: "unavailable" | "internal";
        message: string;
      } | null;
      diagnostics?: {
        path: string;
        line: number;
        col: number;
        endLine?: number;
        endCol?: number;
        message: string;
        code?: string;
        suggestions?: TtcSuggestion[];
        labels?: TtcLabel[];
      }[];
    };
    if (result.backendError) {
      return {
        kind: "unavailable",
        detail: result.backendError.message,
        cause:
          result.backendError.kind === "internal"
            ? "internal"
            : "availability",
      };
    }
    const all = result.diagnostics ?? [];
    let real = fsPath;
    try {
      real = fs.realpathSync(fsPath);
    } catch {
      // keep the raw path; the comparison below still has a chance
    }
    const mine = all.filter((d) => d.path === real || d.path === fsPath);
    // The one-shot parses only this file's lines off stderr, so a failing
    // run whose findings are all in *other* files reads as "could not
    // run" there — and therefore here.
    if (mine.length === 0 && (result.blocked || all.length > 0)) {
      return {
        kind: "unavailable",
        detail: "the check reported only outside this file",
        cause: "availability",
      };
    }
    return {
      kind: "ok",
      diagnostics: mine.map((d) => ({
        line: d.line,
        col: d.col,
        endLine: d.endLine,
        endCol: d.endCol,
        message: d.message,
        code: d.code,
        suggestions: d.suggestions,
        labels: d.labels,
      })),
    };
  }
  return runTypedCheckOnce(compiler, text, fsPath, includeTypes);
}

/** The one-shot `ttc --check-types --overlay` fallback. */
function runTypedCheckOnce(
  compiler: string,
  text: string,
  fsPath: string,
  includeTypes: boolean,
): Promise<ValCheckResult> {
  // Run from the file's own directory so the compiler's paths print as the
  // bare file name (it shows paths relative to the working directory), and
  // a file of the same name elsewhere in the project stays distinguishable.
  const cwd = path.dirname(fsPath);
  const shown = path.basename(fsPath);
  const args = ["--check-types"];
  if (!includeTypes) args.push("--tt-only");
  args.push("--overlay", fsPath, fsPath);

  return new Promise((resolve) => {
    let child: ReturnType<typeof execFile>;
    try {
      child = execFile(
        compiler,
        args,
        {
          cwd,
          timeout: TYPED_CHECK_TIMEOUT_MS,
          maxBuffer: 4 * 1024 * 1024,
        },
        (err, _stdout, stderr) => {
          const diagnostics = parseStderr(String(stderr), shown);
          // Exit code 2 is "could not run, nothing was checked" — a tt-level
          // error left nothing to lower. Anything else with no parseable
          // diagnostic is a missing toolchain or a crash. Both keep whatever
          // the caller already had rather than clearing it.
          const code = (err as { code?: number | string } | null)?.code;
          if (diagnostics.length === 0 && err) {
            resolve({
              kind: "unavailable",
              detail: `${compiler} (${String(code)}): ${String(stderr).trim() || err.message}`,
              cause: code === 101 ? "internal" : "availability",
            });
            return;
          }
          resolve({ kind: "ok", diagnostics });
        },
      );
    } catch (e) {
      resolve({
        kind: "unavailable",
        detail: String(e),
        cause: "availability",
      });
      return;
    }
    // The buffer is the overlay; the compiler reads it from stdin.
    child.stdin?.end(text);
  });
}

/**
 * Read the diagnostics out of a one-shot `ttc --check` run.
 *
 * The compiler renders each one as a header naming the rule and the
 * message, then a location line:
 *
 * ```text
 * error[match-not-exhaustive]: match on variant Shape is not exhaustive
 *  --> src/shape.tt:4:18
 * ```
 *
 * followed by a source excerpt and `= help:` lines this fallback does not
 * need. Positions come from the location line, so nothing here depends on
 * the excerpt's layout. A compiler old enough to predate that rendering
 * still prints `ttc: <file>:<line>:<col>: <msg>`, which is accepted too;
 * `ttc: ` on its own now prefixes usage errors, which name no file and are
 * therefore skipped by the same file check.
 */
export function parseStderr(stderr: string, file: string): TtcDiagnostic[] {
  const diagnostics: TtcDiagnostic[] = [];
  const lines = stderr.split("\n");
  for (const [index, line] of lines.entries()) {
    if (line.startsWith("ttc: ")) {
      const rest = line.slice(5);
      if (!rest.startsWith(file)) continue; // progress logs, other files
      const tail = rest.slice(file.length);
      let m = /^:(\d+):(\d+): (.*)$/.exec(tail);
      if (m) {
        diagnostics.push({
          line: Number(m[1]),
          col: Number(m[2]),
          message: m[3],
        });
        continue;
      }
      m = /^: (.*)$/.exec(tail);
      if (m) diagnostics.push({ line: 0, col: 0, message: m[1] });
      continue;
    }
    const header = /^(?:error|warning)(?:\[([^\]]*)\])?: (.*)$/.exec(line);
    if (!header) continue;
    const location = /^\s*--> (.*):(\d+):(\d+)$/.exec(lines[index + 1] ?? "");
    if (!location || location[1] !== file) continue;
    diagnostics.push({
      line: Number(location[2]),
      col: Number(location[3]),
      message: header[2],
      code: header[1] || undefined,
    });
  }
  return diagnostics;
}

/** Reads a spawn failure: `null` when the process ran and merely reported
 * something. */
export function unusableCompiler(error: unknown): UnusableCompiler | null {
  const code = (error as NodeJS.ErrnoException | null)?.code;
  if (code === "ENOENT") return "missing";
  // The file is there; the operating system would not start it.
  if (code === "EACCES" || code === "EPERM" || code === "ENOEXEC" || code === "EISDIR") {
    return "not-executable";
  }
  return null;
}
