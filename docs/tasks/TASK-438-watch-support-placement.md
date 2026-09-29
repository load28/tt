# TASK-438: Place watch-mode support modules by the whole input set

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: see the `TASK-438:` commit

## Purpose

A watch round that recompiled only a subdirectory file moved the generated
`tt/` support package into that subdirectory and rewrote the file's import to
point there, so a watch round no longer reached the same tree as a one-shot
build (`docs/ai/tt.md`, Workflow).

## Scope

- Included: support-module placement in `compile_jobs` and `watch_mode`, a
  CLI regression test.
- Excluded: output ownership checks, typed watch modes, `-o` builds (whose
  placement was already the output root).

## Decisions

### Decision 1: Make the support root a property of the whole input set

- **Context**: Without `-o`, `std_placement` took the common ancestor of the
  jobs passed to `compile_jobs`. Watch passes only the round's changed files
  and their dependents, so the ancestor depended on which files changed.
- **Alternatives considered**: (a) Always compile every job in watch rounds:
  correct but discards incremental rebuilds. (b) Compute the root once at
  watch start: stale when inputs are added or removed, so it diverges from a
  one-shot build of the current input set. (c) Compute the root from each
  round's full job set and pass it to `compile_jobs` separately from the
  jobs being compiled.
- **Decision and rationale**: (c). A new `support_root(jobs, out_dir)` is
  computed from the full job set (the same set a one-shot build compiles) and
  `compile_jobs` takes it as a parameter, so the subset being compiled can no
  longer influence placement. When the root changes between rounds because
  the input set changed, every output's support specifier changes, so the
  round recompiles all jobs; this keeps the watch result equal to a one-shot
  build of the current inputs.

## Work log

- 2026-09-27: Added `support_root` in `src/main/build.rs`; `std_placement`
  now takes the root; `compile_jobs` takes `support_root`. The one-shot path
  in `src/main/command.rs` computes it from all jobs. `watch_mode` in
  `src/main/output.rs` computes it from each round's full job set and
  recompiles everything when it moves.
- 2026-09-27: Added `watch_places_support_modules_by_the_whole_input_set` to
  `tests/cli.rs`: edits `src/sub/b.tt` and asserts no `src/sub/tt/` and an
  unchanged `../tt/runtime.js` import, then removes `src/a.tt` and `src/a.ts`
  and asserts the support package and import follow the new shared root.

## Issues and resolutions

### Issue 1: Subdirectory watch round relocated the support package

- **Symptom**: `ttc -w src` wrote `src/sub/tt/runtime.ts` and rewrote
  `src/sub/b.ts` to import `./tt/runtime.js` after only `src/sub/b.tt`
  changed.
- **Cause**: `std_placement` used `common_ancestor` of the round's selected
  jobs rather than of the whole build input set.
- **Resolution**: Placement is computed from the full job set and passed to
  `compile_jobs` (Decision 1).

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## Result

Changed `src/main/build.rs`, `src/main/command.rs`, `src/main/output.rs`,
`tests/cli.rs`, `docs/tasks/INDEX.md`, and this record. Watch rounds now
place support modules exactly where a one-shot build of the same inputs does.
