# TASK-801: Keep a snapshot's files in source path order

- **Status**: Complete
- **Started**: 2026-10-14
- **Completed**: 2026-10-14
- **Commit**: `TASK-801: Keep a snapshot's files in source path order`

## Purpose

The nightly incremental sweep (`TT_INCREMENTAL=all cargo test --release
--test incremental`), which has not run in CI since 2026-10-04 because an
earlier step of its job failed (TASK-800), reported 81 samples whose edited
project answered differently from a fresh one. 69 of them differ only in
the order of the emitted files.

## Scope

- Included: the order of `Snapshot::files` and the snapshot's blocked files.
- Excluded: the 12 samples whose answers differ in content; they are
  recorded under Issues.

## Decisions

### Decision 1: "Project order" is source path order

- **Context**: `Snapshot::files` was documented as "in project order", a
  term defined nowhere. `Project::update_with_roots` built the list in
  traversal order: requested, opened, named, and known member files first,
  deferred files when the program joined them, imports as discovered. A
  file the editor had opened or edited therefore came first in an edited
  project and in its path position in a fresh one.
- **Alternatives considered**: Sorting in the test would hide an
  engine answer that depends on the edit history.
- **Decision and rationale**: The projected and blocked files are sorted by
  source path once traversal ends, so the snapshot no longer depends on
  which files were opened or edited; the documentation says "source path
  order".

## Work log

- 2026-10-14: Ran every step of the two nightly jobs that the earlier
  failures had skipped. `incremental`: 81 samples differ; 69 differ only in
  emit order. Decision 1. The two order-only samples rerun
  (`aBackslashRelativeSpecifierIsRewritten.tt#shape.tt#b0d966da1b5b1229`,
  `…#20214f7585e69426`) pass.

## Issues and resolutions

### Issue 1: Emitted files came out in edit-history order

- **Symptom**: `the edited project (-) and a fresh one (+) differ` with the
  same `emit` blocks in another order.
- **Cause**: Decision 1's context.
- **Resolution**: Decision 1.

## Regression test (fails before the fix)

- **Path**: `tests/incremental.rs::an_edited_project_answers_as_a_fresh_one`
  with `TT_INCREMENTAL=all TT_INCREMENTAL_ONLY='tests/cases/compiler/aBackslashRelativeSpecifierIsRewritten.tt#shape.tt#b0d966da1b5b1229'`
- **Observed failure**: the edited project emitted `shape.tt` before
  `a.tt`; a fresh one emitted `a.tt` first.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `TT_INCREMENTAL=all`: no order-only difference remains
- [x] Baseline changes reviewed and committed with the change (none)

## Result

`src/engine/project.rs`, `src/engine/snapshot.rs`.
