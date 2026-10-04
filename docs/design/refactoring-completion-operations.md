# Isolate project completion operations

Task: [TASK-746](../tasks/TASK-746-completion-operations.md). Base: `eb78154e0ee8bdf25eab0f487ce53ddce261332a`.
Program: [behavior-preserving refactoring](behavior-preserving-refactoring.md).

## Boundary and implementation

Move the contiguous completion, triggered_completion, service_completion, pattern_completions, discriminant_candidates, field_candidates, and completion_resolve methods into a private project::completion child with another impl Project. Project and ServiceSession retain all state; children can access the existing private parent methods. Keep source_links, serve, session, and all other operations in project.rs.

## Invariants and exclusions

Preserve trigger handling, probe identity/version, candidate order, cached completion items, fallback conditions, auto-import edits, request/restore order and all error returns. No new completion policy or concurrency.

No algorithms, state ownership, request ordering, public API, dependencies, or
reference expectations change. Inspect the complete diff, compare moved bodies
and reconstruct the parent against the saved base. Unexpected observations block
completion; behavior fixes require separate tasks.

## Allowed files and validation

Production scope: `src/engine/language/project.rs`, `src/engine/language/project/completion.rs`.
Task/index/progress documentation may also change. Base source snapshots and
comparison evidence live under `/tmp/tt-refactor/746`.

Run cargo fmt --check and the native/editor suites; the preceding TASK-745 full gate provides the unchanged source-built base. The final Rust gate must pass on the complete local stack.

The user requested autonomous completion of the remaining roadmap on 2026-10-04.
Slices are prepared sequentially on the local branch, with focused checks before
advancing and the complete required gates before completion/publication. No
intermediate change is merged while its required gates remain pending. Rust
1.98.0 and TypeScript 7.1.0-dev.20260826.1 remain pinned. Do not regenerate baselines.
