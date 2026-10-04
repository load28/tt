# Isolate service target coordinate mapping

Task: [TASK-747](../tasks/TASK-747-target-coordinates.md). Base: `3d1a00c2c68d092353c4fc4a7c4eaf4950c9fe40`.
Program: [behavior-preserving refactoring](behavior-preserving-refactoring.md).

## Boundary and implementation

Move source_edit, TargetUse, TargetCoordinates, target_coordinates, map_target, SharedTarget, authored_shared_binding and map_shared_target into service::targets. The parent re-exports the four consumed entry points. Session/document acquisition remains in serve_doc_only; child calls the same helpers with the same explicit state.

## Invariants and exclusions

Preserve authored/projected provenance, file identity, missing document results, navigation-versus-edit rules, UTF-16 conversion, probe unsplicing, inserted glue, shared-binding occurrence order, and every session/cache side effect.

No algorithms, state ownership, request ordering, public API, dependencies, or
reference expectations change. Inspect the complete diff, compare moved bodies
and reconstruct the parent against the saved base. Unexpected observations block
completion; behavior fixes require separate tasks.

## Allowed files and validation

Production scope: `src/engine/language/service.rs`, `src/engine/language/service/targets.rs`.
Task/index/progress documentation may also change. Base source snapshots and
comparison evidence live under `/tmp/tt-refactor/747`.

Run cargo fmt --check; cargo test --test native --test editor_cases --test content_mapper --test emit_map --test engine_cache. Final full Rust gates are required.

The user requested autonomous completion of the remaining roadmap on 2026-10-04.
Slices are prepared sequentially on the local branch, with focused checks before
advancing and the complete required gates before completion/publication. No
intermediate change is merged while its required gates remain pending. Rust
1.98.0 and TypeScript 7.1.0-dev.20260826.1 remain pinned. Do not regenerate baselines.
