# Complete the refactoring roadmap audit and validation

Task: [TASK-757](../tasks/TASK-757-refactoring-final-audit.md). Base: `ef9dbeff984a10a75b03b429b916e3b62de60828`.
Program: [behavior-preserving refactoring](behavior-preserving-refactoring.md).

## Boundary and implementation

Review the completed extraction series and remaining first-party ownership boundaries, run all default repository gates, and reconcile roadmap and task evidence.

## Invariants and exclusions

No additional production redesign, baseline updates, dependency upgrades, or TASK-732 behavior/performance changes.

No algorithms, state ownership, request ordering, public API, dependencies, or
reference expectations change. Inspect the complete diff, compare moved bodies
and reconstruct the parent against the saved base. Unexpected observations block
completion; behavior fixes require separate tasks.

## Allowed files and validation

Production scope: `docs/design/behavior-preserving-refactoring.md`, `docs/design/refactoring-completion-audit.md`.
Task/index/progress documentation may also change. Base source snapshots and
comparison evidence live under `/tmp/tt-refactor/757`.

Run ./scripts/ci, review the complete production diff, verify unchanged reference expectations, and run task-index and whitespace checks.

The user requested autonomous completion of the remaining roadmap on 2026-10-04.
Slices are prepared sequentially on the local branch, with focused checks before
advancing and the complete required gates before completion and merge. The
user explicitly requested a draft PR after the environment blocked full CI;
publish it with the remaining validation obligation visible. No
intermediate change is merged while its required gates remain pending. Rust
1.98.0 and TypeScript 7.1.0-dev.20260826.1 remain pinned. Do not regenerate baselines.
