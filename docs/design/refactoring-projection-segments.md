# Isolate projection segment representation

Task: [TASK-749](../tasks/TASK-749-projection-segments.md). Base: `d61a93edc7f617af68261ac21dd71bc0170c20e8`.
Program: [behavior-preserving refactoring](behavior-preserving-refactoring.md).

## Boundary and implementation

Move ProjectionSourceSegment, ProjectionSegments and its index/Deref implementations, and ProjectionSegmentKind into projection::segments. Re-export the three types at the same program_syntax visibility. ProjectionBuilder, pending overlays and source traversal remain unchanged.

## Invariants and exclusions

Preserve span-index construction, source/projected units, interval endpoint rules, input order, Deref view, and copied/boundary/placeholder/automatic-semicolon distinctions. Private fields and owners remain private.

No algorithms, state ownership, request ordering, public API, dependencies, or
reference expectations change. Inspect the complete diff, compare moved bodies
and reconstruct the parent against the saved base. Unexpected observations block
completion; behavior fixes require separate tasks.

## Allowed files and validation

Production scope: `src/program_syntax/projection.rs`, `src/program_syntax/projection/segments.rs`.
Task/index/progress documentation may also change. Base source snapshots and
comparison evidence live under `/tmp/tt-refactor/749`.

Run cargo fmt --check, program_syntax unit tests, emit_map and engine_cache suites. Editor and compiler baselines are enforced by the final full Rust gate.

The user requested autonomous completion of the remaining roadmap on 2026-10-04.
Slices are prepared sequentially on the local branch, with focused checks before
advancing and the complete required gates before completion/publication. No
intermediate change is merged while its required gates remain pending. Rust
1.98.0 and TypeScript 7.1.0-dev.20260826.1 remain pinned. Do not regenerate baselines.
