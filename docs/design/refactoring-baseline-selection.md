# Isolate baseline run selection

Task: [TASK-756](../tasks/TASK-756-baseline-selection.md). Base: `8ab26cae865934349c1b1acdbd614bc8b0e73206`.
Program: [behavior-preserving refactoring](behavior-preserving-refactoring.md).

## Boundary and implementation

Move the pure command-line and case-filter selection predicate to a private helper module, retaining environment reads and baseline I/O in the caller.

## Invariants and exclusions

Preserve argument consumption, selection flags, case-filter semantics, tracking headers, baseline comparisons and public helper access.

No algorithms, state ownership, request ordering, public API, dependencies, or
reference expectations change. Inspect the complete diff, compare moved bodies
and reconstruct the parent against the saved base. Unexpected observations block
completion; behavior fixes require separate tasks.

## Allowed files and validation

Production scope: `tests/common/baseline.rs`, `tests/common/baseline/selection.rs`.
Task/index/progress documentation may also change. Base source snapshots and
comparison evidence live under `/tmp/tt-refactor/756`.

Run baseline_tracking Rust tests and baseline-tools Node tests; run the complete Rust gate in the final validation.

The user requested autonomous completion of the remaining roadmap on 2026-10-04.
Slices are prepared sequentially on the local branch, with focused checks before
advancing and the complete required gates before completion/publication. No
intermediate change is merged while its required gates remain pending. Rust
1.98.0 and TypeScript 7.1.0-dev.20260826.1 remain pinned. Do not regenerate baselines.
