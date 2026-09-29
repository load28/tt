# TASK-416: Emit values the lowering plan cannot own as recovery placeholders

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

`emit_mapped` (the `emitMap` server method, `ttc --emit-map`, and the
language-service fallback projection) is documented as infallible for
mid-edit buffers, but it raised internal compiler errors:

- `function f() {\n  const value = try g` →
  `unscheduled expression try reached inline emission` (exit 101);
- a closed function containing `const value = try parse("1").` made
  `completion`, `hover`, `signatureHelp`, `definition`, and `tsDiagnostics`
  answer with the ICE;
- `function read(value = match (1) { … }) {}`, `class C { z = match (a) { … } }`,
  and the repository fixture `tests/fixtures/diagnostic/match-placement/input.tt`
  raised `match reached expression emission without a host rewrite`.

## Scope

- Included: the lowering plan's record of whether an owner model exists
  (`src/evaluation_ir.rs`, `src/evaluation_ir/evaluation.rs`), the target
  rewrite plan and emitter (`src/codegen/core/planning.rs`,
  `src/codegen/core/mod.rs`, `src/codegen/core/emitter/mod.rs`,
  `src/codegen/core/emitter/source.rs`), and `emit_mapped_with_kind`
  (`src/lib/mapped.rs`).
- Excluded: `compile`, `compile_report`, and `compile_projection_report`.
  They never emit a file whose plan fails or refuses a placement. They report
  the refusal as a diagnostic (`TryPlacement`, `MatchPlacement`, or the
  projection failure) before emission starts.

## Decisions

### Decision 1: Treat refused matches the way refused propagations already are

- **Context**: The Evaluation IR already records two typed refusals:
  `unsupported_expression_propagations` and `unsupported_matches`. The target
  plan already turned the first into `recovered_propagations`, which the
  emitter writes as an anchored `undefined` and whose source it claims as
  rewritten. The second had no target-side counterpart, so `emit_mapped`,
  which does not stop at placement diagnostics, sent a refused match into
  expression emission.
- **Alternatives considered**: (a) Lower refused matches through the
  `$tt_recovery` expression closure. Match placement exists precisely because
  a closure changes `this`, `arguments`, `await`, and parameter scope
  (TASK-407), so the service would type a program that differs from the
  source. (b) Splice placeholder text into the source and re-emit. That is
  what `compile_projection_report` does, but it needs the diagnostics
  `emit_mapped` deliberately skips.
- **Decision and rationale**: `TargetRewritePlan` carries
  `recovered_matches` from the plan's `unsupported_matches`. The emitter emits
  each one as an anchored `undefined` (`AnchorKind::Match`, from the head to
  the extent) and claims its source span as rewritten. This matches
  propagation recovery, and it matches the typed path's placeholder
  (`compile_projection_report` replaces the same `MatchPlacement` span with a
  placeholder expression).

### Decision 2: Make "no owner model" an explicit plan state instead of an empty plan

- **Context**: When the buffer's TypeScript does not parse, `lowering_plan`
  fails. `emit_mapped_with_kind` then used `LoweringPlan::default()`, the same
  value that means "this file needs no host lowering". The emitter could not
  tell the two apart, so an expression `try` reached the ICE that guards
  planner bugs in the compile path.
- **Alternatives considered**: Remove the ICE and always emit a placeholder
  for an unscheduled propagation. That hides real planner defects in
  `compile`, which AGENTS.md contract 3 forbids.
- **Decision and rationale**: `LoweringPlan::without_owner_model()` records
  that no owner model exists. `emit_mapped_with_kind` uses it when planning
  fails. Only in that state does the emitter emit an unowned expression
  propagation as the same anchored `undefined` placeholder, and it records
  the construct's span as rewritten source. Built plans keep the ICE.
  Statement-level `try` (`Statement::Propagate`) and the existing statement
  match recovery are unchanged.

## Work log

- 2026-09-27: Reproduced all three ICE shapes with `ttc --emit-map` and through
  `Project::{completion, hover, signature_help, definition, service_diagnostics}`.
- 2026-09-27: Implemented both decisions. Added
  `tests/emit_map.rs::values_the_plan_cannot_own_emit_as_anchored_placeholders`
  (mid-edit `try`, a `try` in an unparsable body, a parameter-default match,
  and a class-field match). The test checks the mapping invariants, an
  `undefined` anchor at the construct, and that no mapping enters the
  construct. Added `tests/native/cases_03.rs::service_requests_answer_over_values_the_plan_cannot_own`,
  which drives the five service requests on the three reported buffers.
  Both tests fail before the change (ICE) and pass after it.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test --lib --test emit_map --test compile --test snapshot --test cli`
- [x] `TTC_REQUIRE_TSGO=1 cargo test --test native service_requests_answer_over_values_the_plan_cannot_own`
- [x] `TTC_REQUIRE_TSGO=1 cargo test` (the full run at the end of TASK-421)

## Result

`ttc --emit-map` now emits the three reported shapes and exits 0, and every
service request answers over them. Changed `src/evaluation_ir.rs`,
`src/evaluation_ir/evaluation.rs`, `src/codegen/core/planning.rs`,
`src/codegen/core/mod.rs`, `src/codegen/core/emitter/mod.rs`,
`src/codegen/core/emitter/source.rs`, `src/lib/mapped.rs`, `tests/emit_map.rs`,
and `tests/native/cases_03.rs`.
