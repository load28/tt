# TASK-509: Report `--types` writes per file and settle editor sidecars from the report

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: see `git log --grep TASK-509`

## Purpose

The repository owner's review of PR #130 found that the `ttc --types` exit
status could not describe what happened to the files it writes, and that the
VS Code extension read success into two outcomes that were failures:

- (2) `write_declarations(...)?` turned a write failure into an error, and
  `typed_check_mode` exited 1 — the same status as "type diagnostics were
  reported and every file was written". `sidecar.ts` treated exit 1 as
  `written`, so an unrefreshed sidecar was reported as refreshed and the
  self-write ledger was settled for files that were not written. A failure
  midway through several declarations left some files new and some old, and
  nothing said which.
- (3) `sidecar.ts` computed `err.code ?? 1`, so an `execFile` killed by a
  signal (`err.code === null`, `err.signal` set) counted as exit 1 and
  therefore as `written`.

## Scope

- Included: the `--check-types`/`--types` exit-status contract, a
  machine-readable per-file write report (`--json-report`), per-file write
  handling in `write_declarations`, the extension's sidecar refresh and
  write ledger, CLI help, `docs/ai/tt.md`, the extension README, the
  changelog, and Rust and extension regression tests.
- Excluded: `--server` (it does not write declarations), `--watch` report
  output, and other CLI modes' exit statuses.

## Decisions

### Decision 1: A structured per-file report is the contract; exit statuses summarize it

- **Context**: The editor needs to know, per file, whether the refresh
  landed, including partial output when several writes fail midway.
- **Alternatives considered**:
  - Distinct exit codes only. A code can say "some writes failed" but not
    which ones; the editor would have to re-derive the answer from file
    timestamps or contents, which races with other writers.
  - A report file named by a flag (`--report <path>`). It adds a second
    write that can itself fail and a temporary file the editor must clean.
  - One JSON object on stdout. `--types` prints nothing on stdout (all
    diagnostics go to stderr), and the tooling modes (`--symbols`,
    `--dependencies`, `--emit-map`, `--server`) already answer with JSON on
    stdout.
- **Decision and rationale**: `--types --json-report` prints one object,
  `{"checked", "diagnostics", "written": [absolute paths], "failed":
  [{"path", "error"}]}`, where every file the run tried to write appears in
  exactly one list. Absolute lexical paths make the report independent of
  the working directory. The flag is limited to one `--types` run: it is
  rejected with `--watch` (no single final answer) and in modes that write
  nothing. No object means nothing was written — an invalid command line, an
  internal compiler error, or a killed process.

### Decision 2: Document four typed exit statuses

- **Context**: The exit status still has to be honest for shells and CI.
- **Alternatives considered**: Keep 1 for write failures and rely on the
  report; or add a status only for "no file written". Both leave a status
  that means two things.
- **Decision and rationale**: 0 clean (every file written), 1 diagnostics
  reported (every file written all the same), 2 the check could not run
  (nothing written, earlier outputs stand), 3 `--types` could not write one
  or more files (the rest were; stderr and the report name each). A project
  that cannot be opened and a check request that errors now exit 2 instead
  of 1, because nothing was checked or written; that is what 2 already meant.
  An invalid command line keeps its exit 1 and prints no report. The list is
  in `ttc --help` under "Exit status" and in `docs/ai/tt.md`.

### Decision 3: Attempt every write and record each result

- **Context**: `write_declarations` stopped at the first error.
- **Decision and rationale**: The output plan (standard-library files, each
  module's declaration and map) is built first. A collision between
  declaration targets is still found before anything is written and fails
  every planned file. Otherwise each file is written independently and
  recorded in a `WriteOutcome`; a map whose declaration was not written is
  not written either (a new map beside an old declaration would mismatch it)
  and is recorded as failed with that reason. Standard-library declarations
  now use the same atomic `replace_file` as the sidecars.

### Decision 4: The editor settles the ledger from the report, per file

- **Context**: PR #129 (branch `task-388-compiler-editor-structural-repairs`,
  commit 2bfec45) fixed the signal case by dropping the `?? 1` fallback. Its
  record is numbered TASK-388, which collides with this branch's TASK-388,
  so the behaviour is integrated here rather than cherry-picked.
- **Decision and rationale**: `sidecar.ts` runs `ttc --types --json-report`.
  A process killed by a signal, an exit status outside 0–3, a spawn error,
  or stdout that is not a valid report is a failed refresh that wrote
  nothing. Otherwise a file counts as written only when the report names it
  (compared by resolved and canonical path); `WriteLedger.settle` takes that
  list, so the ledger owns exactly the files that were written. A report
  with `checked: false` is a failure that keeps the last sidecar. The
  `failed` result now carries the files that were written, so a partial
  refresh is visible to callers and logged with each file's reason. The
  #129 regression test (a compiler that kills itself with `SIGTERM`) is
  carried over.

## Work log

- 2026-09-28: Read the review items, `src/main/typed.rs`,
  `src/main/command.rs`, `editors/vscode/server/src/sidecar.ts`, and the
  #129 diff (`git diff origin/main...origin/task-388-compiler-editor-structural-repairs`).
- 2026-09-28: Added `WriteOutcome` and the exit-status mapping on
  `TypedReport`; rewrote `write_declarations` to plan, then write per file;
  routed project-open and check errors to status 2; added `--json-report`
  parsing and its combination rules; documented both in `ttc --help`.
- 2026-09-28: Rewrote the extension's `run`/`writeSidecar` around the report
  and changed `WriteLedger.settle` to take the written files.
- 2026-09-28: Added `tests/native/cases_07.rs` (exit 1 with a full report,
  exit 3 with a partial write, exit 2 leaving an earlier sidecar, no stdout
  without the flag), a CLI combination test in `tests/cli.rs`, and extension
  tests for signal termination, a missing report, full, partial, unnamed and
  unchecked reports, and a real partial write.
- 2026-09-28: Updated `docs/ai/tt.md`, `editors/vscode/README.md`, and
  `CHANGELOG.md`.

## Issues and resolutions

### Issue 1: Declaration maps named an absolute source under a relative `-o`

- **Symptom**: Two extension tests that run `ttc --types src` (relative
  output directory) failed: the refresh found no sidecar, because the map's
  `sources` was `../..///home/.../src/a/notice.tt`.
- **Cause**: The rewrite built the sidecar (and its relative source path)
  before creating the output directory. `relative_path` canonicalizes both
  sides only when both exist, so a missing directory mixed a relative base
  with an absolute source.
- **Resolution**: Create the directory before building the sidecar, as the
  previous code did. The Rust exit-1 test now writes under a relative `-o`
  and asserts the map's `sources`.

### Issue 2: The disk filled during verification

- **Symptom**: `ENOSPC` while the test suite and the extension stage ran.
- **Cause**: About 1,500 `/tmp/tt-*` directories (roughly 124 MB each) left
  behind by earlier extension-suite runs.
- **Resolution**: Removed the leftovers older than an hour and reran the
  gates. The leak predates this task and is not changed here.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] `./scripts/ci extension`
- [x] Exit statuses checked by hand: 1 with diagnostics and every file
  written, 3 with a blocked map path (report names the failure), 2 for a
  missing input (`checked: false`), 1 and no report for rejected flag
  combinations.

## Result

Changed files: `src/main.rs`, `src/main/command.rs`, `src/main/typed.rs`,
`tests/cli.rs`, `tests/native.rs`, `tests/native/cases_07.rs`,
`editors/vscode/server/src/sidecar.ts`,
`editors/vscode/server/src/test/sidecar.test.ts`, `editors/vscode/README.md`,
`docs/ai/tt.md`, `CHANGELOG.md`, `docs/tasks/INDEX.md`, and this record.
`ttc --types` now states per file what it wrote, its exit status separates
"reported" from "could not write", and the extension no longer reports or
records an unrefreshed sidecar as written.
