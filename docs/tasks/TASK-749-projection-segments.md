# TASK-749: Isolate projection segment representation

- **Status**: In progress
- **Started**: 2026-10-04
- **Completed**: —
- **Commit**: —

## Purpose

Move ProjectionSourceSegment, ProjectionSegments and its index/Deref implementations, and ProjectionSegmentKind into projection::segments. Re-export the three types at the same program_syntax visibility. ProjectionBuilder, pending overlays and source traversal remain unchanged.

## Scope

- Included: `src/program_syntax/projection.rs`, `src/program_syntax/projection/segments.rs`.
- Excluded: Behavior changes, public API changes, baseline updates, and TASK-732.

## Decisions

### Decision 1: Extract the existing responsibility without redesign

- **Context**: The roadmap identifies a coherent operation mixed with its caller.
- **Alternatives considered**: Keeping the boundary implicit; combining the move
  with algorithm or state redesign, which would make equivalence harder to review.
- **Decision and rationale**: Preserve definitions and callers where possible,
  using the smallest visibility required by the existing consumers. See the
  [detailed brief](../design/refactoring-projection-segments.md).

## Work log

- 2026-10-04: Started from local base `d61a93edc7f617af68261ac21dd71bc0170c20e8` on
  `refactor/remaining-roadmap`. Preserved base source under
  `/tmp/tt-refactor/749` before production edits.

- 2026-10-04: Exact segment-definition block and inverse parent reconstruction passed. Formatting, program_syntax unit tests (34, 385 intentionally filtered), emit_map (23), and engine_cache (10) passed; zero failures/ignored tests. /tmp/tt-refactor/749/focused.log. Final compiler/editor baselines remain required.

## Issues and resolutions

None.

## Regression test (fails before the fix)

Not applicable: Behavior-preserving extraction; no bug is fixed.

## Verification

- [x] Moved definitions and parent changes reviewed.
- [x] Focused checks: Run cargo fmt --check, program_syntax unit tests, emit_map and engine_cache suites. Editor and compiler baselines are enforced by the final full Rust gate.
- [ ] Applicable complete repository gates.
- [ ] Reference expectations unchanged; task-index and whitespace checks.

## Result

Implementation and focused validation complete; final repository gates remain pending.
