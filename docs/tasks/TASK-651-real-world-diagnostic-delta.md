# TASK-651: Compare the merge base's diagnostics and output with the change's, over real tt programs

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-651`

## Purpose

The baseline suites show what a change does to the programs written as
tests. They do not show what it does to programs written as programs: the
practical fixtures, the examples on the website, the project `create-tt`
scaffolds. TypeScript runs its pull requests against real projects and
reports every error that appears or disappears (the user tests and
typescript-error-deltas). This task does the same for tt, and makes two
outcomes fail the pull request.

## Scope

- Included: `scripts/diagnostic-delta`, the `delta` job in
  `.github/workflows/ci.yml` (pull requests), and `CONTRIBUTING.md` ("The
  real-world diagnostic delta"). Also, per the no-comment rule of this work:
  the comments TASK-648 and TASK-650 added to `scripts/ci` and
  `.github/workflows/ci.yml` were removed here (their records explain the
  steps).
- Excluded: projects outside this repository (TypeScript clones popular
  repositories; tt has none yet), editor answers (TASK-639 and TASK-647
  cover them), and performance (`scripts/bench-compare`).

## Sources modelled

- microsoft/typescript-error-deltas at
  `0d5c27afbda5c0000b08f76ce4f2b616e5108928`: `README.md` (compile each
  repository with the current TypeScript and with the pull request's, and
  report the errors only the new one issues); `src/main.ts` (lines 776 to
  870: build with the new compiler, then per project `newlyReported` and
  `newlyUnreported` against the old compiler's errors, skipping projects
  whose errors are unchanged; lines 392 to 545: the language-server run
  that reports crashes, grouped by stack hash).
- microsoft/TypeScript wiki at `966988bc`, `Triggering-TypeScript-Bot.md`
  (`user test this`: the user suite "against the PR and against main ...
  The bot will post a summary comment comparing results from the two") and
  `How-the-User-Tests-Work.md` (programs kept small and buildable with one
  compiler invocation).
- This repository's `scripts/bench-compare` (TASK-225): both revisions
  built and run on one machine in one run, the base in a detached worktree
  under `.tt-dev/` that is removed on exit.

## Decisions

### Decision 1: The pinned programs are the repository's own real programs, run from head's copy

- **Context**: The request named the practical diagnostic fixtures, the
  website examples, integration examples, and the `create-tt` templates.
- **Alternatives considered**: Each revision's own copy of the programs
  (then a change to a fixture reads as a compiler change); a separate
  corpus directory (a copy that drifts from what users see).
- **Decision and rationale**: The script materializes the programs once,
  from `HEAD`, under `.tt-dev/delta/programs/` (inside the repository, so
  `node_modules/typescript` resolves as for any project): each directory of
  `tests/fixtures/practical-diagnostics/`; `tests/fixtures/mixed-source-matrix`
  and `tests/fixtures/mixed-source-runtime`, the projects the integration
  tests build (the integration tests' other programs are inline strings in
  Rust, with no project around them); every topic of
  `website/src/content.json` that `website/scripts/highlight.ts` highlights
  as tt or ttx (14 examples, each with a strict bundler `tsconfig.json`);
  and the project `createProject()` of `packages/create-tt/src/installer.js`
  writes. Both binaries run on the same files. 21 programs today.

### Decision 2: What is compared

- **Decision and rationale**: For each program, `ttc --check .`,
  `ttc --check-types .`, and `ttc --no-banner --jobs 1 --out-dir <out> .`
  (with every emitted file appended), each as exit status plus output, the
  program directory and output directory replaced by `$DIR` and `$OUT`, and
  `RUST_BACKTRACE=0` so a crash reads the same on every machine. Unlike
  error-deltas, the whole output is compared rather than a set of error
  codes: a changed message, span, help, or emission is a user-visible
  change too. The report is Markdown: a table of every run and, for each
  change, a line diff (60 lines at most).

### Decision 3: Two failures, everything else a report

- **Context**: error-deltas only reports. The request asked for a report,
  with failure on new ICEs or on changed output without baseline changes,
  as designed here.
- **Alternatives considered**: Report only (a crash on a real program would
  merge); fail on any change (every intended fix would need an override).
- **Decision and rationale**: The script exits 1 when a run of `HEAD` is an
  internal compiler error (exit 101, or "internal compiler error" in its
  output) and the base's same run was not, and when any run's output
  changed while `git diff <base>..HEAD` touches no baseline
  (`tests/baselines/reference/**` or `tests/fixtures/**/expected.*`). The
  second is TASK-636's rule seen from the outside: a change in behaviour
  comes with a case that pins it. A fixed ICE is reported as `ICE fixed`.

### Decision 4: CI cost

- **Decision and rationale**: A `delta` job on pull requests only, in
  parallel with the others, with the full history (`fetch-depth: 0`) and
  `--base origin/$GITHUB_BASE_REF`, so a pull request into `release-X.Y`
  compares with that branch. The base is built with
  `--config profile.dev.debug=0` in its worktree's own target directory
  (no debug information: less disk and time, the same diagnostics); head
  reuses the cached debug build. Measured locally with two build jobs: the
  base build takes 71 to 74 seconds and the whole script 97 seconds (the
  126 runs about 25 of them). A runner without a cached base build pays a
  full dependency build for the base. The report goes to the job summary and the `diagnostic-delta`
  artifact.

## Work log

- 2026-09-30: Read error-deltas' `README.md` and `src/main.ts` at
  `0d5c27af` and the two wiki pages.
- 2026-09-30: Wrote `scripts/diagnostic-delta`, the CI job, and the
  `CONTRIBUTING.md` section.
- 2026-09-30: Ran it against `HEAD~1` (clean), then with a deliberate
  crash in the head build (below), then again clean.

## Issues and resolutions

None.

## Regression test (fails before the fix)

Not applicable: this task adds a comparison script and a CI job and fixes
no compiler bug. Its failures are shown under Verification.

## Verification

- [x] `node scripts/diagnostic-delta --base HEAD~1`: 21 programs, 63 runs,
  0 changed, 0 new ICEs; exit 0 in 97 seconds; `.tt-dev/delta-base` and its worktree
  entry removed afterwards.
- [x] With a temporary `panic!` in `src/main.rs` for `--check` in the
  `create-tt-starter` directory: "1 new internal compiler error(s):
  create-tt-starter --check" and "1 output(s) changed, but the change
  touches no baseline"; the row reads `0 | 101 | NEW ICE`; exit 1. The
  panic was reverted.
- [x] `node --check scripts/diagnostic-delta`; the workflow parses as YAML
  with the new `delta` job.
- [x] The full gate ran over the final tree of TASK-647 to TASK-651 (see
  below).

## Result

Changed files: `scripts/diagnostic-delta`, `.github/workflows/ci.yml`,
`scripts/ci`, `CONTRIBUTING.md`, `docs/tasks/INDEX.md`, and this record.
