# TASK-354: A missing toolchain removes the refinement, not the compilation

- **Status**: Complete
- **Started**: 2026-09-10
- **Completed**: 2026-09-10
- **Commit**: `TASK-354: fix(compiler): degrade contextual refinement without a toolchain`

## Purpose

TASK-353 marked every join site as a contextual slot so an inferred
annotation could reach it. That made the standalone contextual pass run for
every file holding a tt construct, and the pass fails hard when no
TypeScript is installed — so `ttc --check` and `ttc -p`, which are
documented as needing none, stopped working without one.

## Scope

- Included: how a failed contextual pass is classified by its two callers,
  and the emit fixture TASK-353 left stale
- Excluded: what the contextual pass computes when it does run

## Decisions

### Decision 1: An unavailable backend is not a compile error

- **Context**: `standalone` returns a `Failure` carrying a kind. Both
  callers treated every kind as fatal, which was harmless while the pass
  only ran for files that already needed a checker.
- **Alternatives considered**: Stop marking join sites (this undoes the
  inferred-storage fix TASK-353 made for the evolving-array case); resolve a
  toolchain from somewhere else (there is only one place a project's
  TypeScript comes from).
- **Decision and rationale**: A contextual annotation refines the *type* of
  a generated storage slot; the emitted program is correct without one. So
  `FailureKind::Unavailable` returns the unrefined emit and
  `FailureKind::Internal` — a backend that ran and broke its own contract —
  stays an error. This is the rule the typed pass already follows
  (`docs/design/compiler-core.md` §7).

## Work log

- 2026-09-10: Reviewed TASK-353 against the seven reproductions it claims.
  All seven behave as claimed, and both regressions it found in TASK-352
  (an output path escaping `-o`, duplicate inputs racing over one staging
  file) are fixed.
- 2026-09-10: `./scripts/ci` failed at the `rust` stage on the TASK-353
  head: `tests/fixtures/emit/try-and-result/expected.ts` was not
  regenerated with the other eleven.
- 2026-09-10: Found that any file holding a tt construct fails `--check`
  and `-p` with "no TypeScript compiler found" when none is installed;
  confirmed against the previous head, which compiles the same files.
- 2026-09-10: Classified the failure at both call sites and pinned the
  contract with a test that runs from a directory outside this checkout —
  the toolchain is looked up from the process's location as well as the
  file's, so a case inside `target/` resolves the repository's own install
  and proves nothing.

## Issues and resolutions

### Issue 1: `--check` and `-p` demanded a TypeScript installation

- **Symptom**: In a directory with no `node_modules` above it,
  `ttc --check shape.tt` and `ttc -p shape.tt` exited 1 with
  `error[other]: no TypeScript compiler found` for any file holding a tt
  construct. The previous head compiled the same files and exited 0. The
  `--help` text says `--check` "needs no TypeScript", and `-p` is what the
  bundler adapters call.
- **Cause**: Marking every join site as a contextual slot made
  `contextual::standalone` run for those files, and both callers turned any
  failure from it into a compile error.
- **Resolution**: An unavailable backend returns the unrefined emit. The
  emitted output is byte-identical to the previous head's for those files.

### Issue 2: One emit fixture was left stale

- **Symptom**: `cargo test` failed with
  `tests/fixtures/emit/try-and-result/expected.ts is out of date`.
- **Cause**: The inferred join annotation reaches that fixture too; eleven
  others were regenerated and this one was not.
- **Resolution**: Regenerated and the diff read — the slot gains the
  annotation its `TResult` return type gives it, and the rest is unchanged.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci` — all six stages pass.
- The seven TASK-353 reproductions still behave as claimed after this
  change: the `if let` case prints 105, the template literal, `result`-owned
  `try`, and empty-array cases check clean, `val` reports all four
  assignment forms, watch reports the missing input once, and the editor
  removes its per-request directory.

## Result

`ttc --check` and `ttc -p` answer again with no TypeScript installed, and
the emitted output for those files matches the previous head byte for byte.
The gate is green across all six stages.

Left open: the emit fixtures now depend on an installed TypeScript —
`cargo test` in a checkout that has not run `npm ci` fails on
`contextual-composed-match`. That is a contract question for the fixtures
(whether an emit snapshot may encode checker-derived types) rather than a
defect in this change, and it wants its own task.
