# TASK-625: Keep another file's compiler failure out of the printed file

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-625`

## Purpose

`ttc -p a.tt` stopped with exit 101 and an internal compiler error report
naming `a.tt` when another `.tt` file of the same project hit an internal
compiler error. `-p` refines `a.tt`'s generated storage against its project,
which projects every source of the project (`Reuse::reconcile` in
`src/typescript/contextual.rs`); that projection ran under `a.tt`'s
`working_on` frame and without a boundary, so the other file's failure was
reported as `a.tt`'s and ended `a.tt`'s compile.

## Scope

- Included: the project projections `-p`'s refinement computes, the
  projections of a `--check-types`/editor snapshot
  (`src/engine/project.rs`), a debug-only failure point for tests, a CLI
  test, and `docs/ai/tt.md`.
- Excluded: making a snapshot survive one of its own files' failure. Every
  file of a `--check-types` run is the run's subject, so a failure in any
  of them is the run's failure; it is now named correctly (Decision 2).

## Decisions

### Decision 1: Another file's projection is its own unit of work

- **Context**: `a.tt`'s output does not depend on another file's
  projection succeeding: a file whose projection is withheld (it has a tt
  error) is already left out of the modules sent to the checker, and
  `a.tt` is typed without it. An internal compiler error in that other file
  is the same absence, caused by the compiler instead of the file.
- **Alternatives considered**: (a) Leave it: a bug in a file the user did
  not ask about stops the one they did, under the wrong name. (b) Only name
  the file (`working_on`): the report is right, but `a.tt` still fails.
  (c) Name the file and bound its projection with `ice::catching`, the
  boundary `--server` and the CLI already put around a unit of work: the
  report is printed by the panic hook where the failure happens, under the
  file's name, and the file is left without a projection.
- **Decision and rationale**: (c). The report still reaches the user,
  unchanged, and names the file at fault; nothing is suppressed. `a.tt`'s
  exit status is `a.tt`'s: it compiles as it does when the other file has a
  tt error.

### Decision 2: A snapshot names the file it was projecting

- **Context**: `Project` projects each file of a snapshot the same way,
  with no `working_on` frame, so a failure there named no file or the
  caller's.
- **Alternatives considered**: Bound each snapshot projection as in
  Decision 1. For `--check-types` the run is about every file, and a
  snapshot with a missing file would report its importers' errors as the
  checker's.
- **Decision and rationale**: Name the file (`working_on`) and keep the
  failure the run's.

### Decision 3: A named failure point per file

- **Context**: After TASK-621 and TASK-622 no known input fails, and the
  behaviour of a failure can only be observed by failing. `TTC_PANIC_FOR_TEST`
  (TASK-214) is the debug-only seam for that.
- **Decision and rationale**: Both projection sites raise at
  `projection:<file name>`, so a test fails exactly one file's projection.
  A release build has no path to it.

## Work log

- 2026-09-30: Found the sibling projections in `Reuse::reconcile`, run
  under the requested file's `working_on` frame; confirmed no live input
  still fails after TASK-621/622.
- 2026-09-30: `src/typescript/contextual.rs`: each changed projection runs
  under its own `working_on` frame inside `ice::catching`, with the
  failure point. `src/engine/project.rs`: each snapshot projection runs
  under its file's `working_on` frame, with the failure point.
- 2026-09-30: Test
  `a_siblings_compiler_bug_is_its_own_and_the_printed_file_still_compiles`
  (`tests/cli.rs`): with `b.tt`'s projection failing, `ttc -p a.tt` exits 0,
  prints `a.tt`'s output, and the report names `b.tt` and not `a.tt`;
  `--check-types .` exits 101 naming `b.tt`. It fails without the change.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 cargo test`
- [x] `cd editors/vscode && npm run compile && node --test "server/out/test/*.test.js" "client/out/test/*.test.js"`
- [x] `node scripts/check-task-index`

## Result

Changed `src/typescript/contextual.rs`, `src/engine/project.rs`,
`tests/cli.rs`, `docs/ai/tt.md`, `docs/tasks/INDEX.md`, and this record.
