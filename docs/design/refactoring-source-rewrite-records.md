# Isolate source rewrite records and local edit helpers

Task: [TASK-750](../tasks/TASK-750-source-rewrite-records.md). Base: `c995f25f45540662865d6a7c468986c8ad9b8f1b`.
Program: [behavior-preserving refactoring](behavior-preserving-refactoring.md).

## Boundary and implementation

Move OwnerSlotRewrite, ForInitializerPropagationRewrite, ArrowReturnRewrite, DeclaratorSplitRewrite, ComposeRewrite, LoopTestRewrite, ComposeAction, CallCompletionPlan, ComposeValue, SourceReplacement with written, LocalSourceEdit and ResultReturnBoundary, plus declarator_separator, compound_assignment_operator, discarded_operand_comma, discarded_operand_commas and compound_assignment_frames into planning::rewrites. Keep TargetRewritePlan::build and evaluation scheduling unchanged.

## Invariants and exclusions

Preserve every field, enum variant, generated string, source span, trivia scan, error branch, iterator order, and rewritten-operation/source-preservation decision. The two formerly private aggregate helpers are visible only to planning; other symbols retain core-module visibility. Do not split scheduling or rename records.

No algorithms, state ownership, request ordering, public API, dependencies, or
reference expectations change. Inspect the complete diff, compare moved bodies
and reconstruct the parent against the saved base. Unexpected observations block
completion; behavior fixes require separate tasks.

## Allowed files and validation

Production scope: `src/codegen/core/planning.rs`, `src/codegen/core/planning/rewrites.rs`.
Task/index/progress documentation may also change. Base source snapshots and
comparison evidence live under `/tmp/tt-refactor/750`.

Run cargo fmt --check and compile, snapshot, practical_diagnostics and emit_map tests. Full compiler/runtime baselines and Rust gates are required before final completion.

The user requested autonomous completion of the remaining roadmap on 2026-10-04.
Slices are prepared sequentially on the local branch, with focused checks before
advancing and the complete required gates before completion/publication. No
intermediate change is merged while its required gates remain pending. Rust
1.98.0 and TypeScript 7.1.0-dev.20260826.1 remain pinned. Do not regenerate baselines.
