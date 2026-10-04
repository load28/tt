# TASK-747: Isolate service target coordinate mapping

- **Status**: In progress
- **Started**: 2026-10-04
- **Completed**: —
- **Commit**: —

## Purpose

Move source_edit, TargetUse, TargetCoordinates, target_coordinates, map_target, SharedTarget, authored_shared_binding and map_shared_target into service::targets. The parent re-exports the four consumed entry points. Session/document acquisition remains in serve_doc_only; child calls the same helpers with the same explicit state.

## Scope

- Included: `src/engine/language/service.rs`, `src/engine/language/service/targets.rs`.
- Excluded: Behavior changes, public API changes, baseline updates, and TASK-732.

## Decisions

### Decision 1: Extract the existing responsibility without redesign

- **Context**: The roadmap identifies a coherent operation mixed with its caller.
- **Alternatives considered**: Keeping the boundary implicit; combining the move
  with algorithm or state redesign, which would make equivalence harder to review.
- **Decision and rationale**: Preserve definitions and callers where possible,
  using the smallest visibility required by the existing consumers. See the
  [detailed brief](../design/refactoring-target-coordinates.md).

## Work log

- 2026-10-04: Started from local base `3d1a00c2c68d092353c4fc4a7c4eaf4950c9fe40` on
  `refactor/remaining-roadmap`. Preserved base source under
  `/tmp/tt-refactor/747` before production edits.

- 2026-10-04: Exact child definitions and inverse parent reconstruction passed after formatting; all target-mapping callers are unchanged. Focused content_mapper, editor_cases, emit_map, engine_cache, and native suites exited 0 with no failures/ignored/filtered tests; /tmp/tt-refactor/747/focused.log. Full Rust gates remain pending.

## Issues and resolutions

None.

## Regression test (fails before the fix)

Not applicable: Behavior-preserving extraction; no bug is fixed.

## Verification

- [x] Moved definitions and parent changes reviewed.
- [x] Focused checks: Run cargo fmt --check; cargo test --test native --test editor_cases --test content_mapper --test emit_map --test engine_cache. Final full Rust gates are required.
- [ ] Applicable complete repository gates.
- [ ] Reference expectations unchanged; task-index and whitespace checks.

## Result

Implementation and focused validation complete; final repository gates remain pending.
