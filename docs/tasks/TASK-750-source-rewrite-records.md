# TASK-750: Isolate source rewrite records and local edit helpers

- **Status**: In progress
- **Started**: 2026-10-04
- **Completed**: —
- **Commit**: —

## Purpose

Move OwnerSlotRewrite, ForInitializerPropagationRewrite, ArrowReturnRewrite, DeclaratorSplitRewrite, ComposeRewrite, LoopTestRewrite, ComposeAction, CallCompletionPlan, ComposeValue, SourceReplacement with written, LocalSourceEdit and ResultReturnBoundary, plus declarator_separator, compound_assignment_operator, discarded_operand_comma, discarded_operand_commas and compound_assignment_frames into planning::rewrites. Keep TargetRewritePlan::build and evaluation scheduling unchanged.

## Scope

- Included: `src/codegen/core/planning.rs`, `src/codegen/core/planning/rewrites.rs`.
- Excluded: Behavior changes, public API changes, baseline updates, and TASK-732.

## Decisions

### Decision 1: Extract the existing responsibility without redesign

- **Context**: The roadmap identifies a coherent operation mixed with its caller.
- **Alternatives considered**: Keeping the boundary implicit; combining the move
  with algorithm or state redesign, which would make equivalence harder to review.
- **Decision and rationale**: Preserve definitions and callers where possible,
  using the smallest visibility required by the existing consumers. See the
  [detailed brief](../design/refactoring-source-rewrite-records.md).

## Work log

- 2026-10-04: Started from local base `c995f25f45540662865d6a7c468986c8ad9b8f1b` on
  `refactor/remaining-roadmap`. Preserved base source under
  `/tmp/tt-refactor/750` before production edits.

- 2026-10-04: Exact moved definitions and inverse reconstruction of planning.rs passed after rustfmt. compile (186), emit_map (23), practical_diagnostics (1), snapshot (4) passed with zero failures/ignored/filtered; /tmp/tt-refactor/750/focused.log. TargetRewritePlan::build is byte-for-byte unchanged.

## Issues and resolutions

None.

## Regression test (fails before the fix)

Not applicable: Behavior-preserving extraction; no bug is fixed.

## Verification

- [x] Moved definitions and parent changes reviewed.
- [x] Focused checks: Run cargo fmt --check and compile, snapshot, practical_diagnostics and emit_map tests. Full compiler/runtime baselines and Rust gates are required before final completion.
- [ ] Applicable complete repository gates.
- [ ] Reference expectations unchanged; task-index and whitespace checks.

## Result

Implementation and focused validation complete; final repository gates remain pending.
