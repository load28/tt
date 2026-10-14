# TASK-803: Drop a suggestion that covers a construct's glue again

- **Status**: Complete
- **Started**: 2026-10-14
- **Completed**: 2026-10-14
- **Commit**: `TASK-803: Drop a suggestion that covers a construct's glue again`

## Purpose

The nightly editor matrix (`TT_MATRIX_CASES=all cargo test --release
--test editor_cases`) reported, for every `match_*_awaitOperand_*` case, a
`TS80007` hint ("'await' has no effect on the type of this expression")
over `await (match ...)` that the twin comparison had not shown before.
`docs/ai/tt.md` ("Editor suggestions") says such a suggestion is dropped.

## Scope

- Included: the editor's service-diagnostic filter for suggestions.
- Excluded: errors and warnings, whose relocated mapping TASK-795 Decision 5
  introduced and which stay as they are.

## Decisions

### Decision 1: A suggestion whose span holds a construct's whole glue is dropped

- **Context**: TASK-795 Decision 5 (`mapper::relocated_origin`) maps a
  span of copied text and reads of relocated operands back to the source
  text they stand for, as `Exact`. A lowered value's slot read is such a
  relocated operand, so `await ($tt_v0)` mapped exactly onto
  `await (match ...)`, and the suggestion filter, which only required an
  exact origin, let the hint through.
- **Alternatives considered**: Not recording value-slot reads as relocated
  operands would undo TASK-795's mapping for errors on them; listing the
  difference would contradict the documented behaviour.
- **Decision and rationale**: An Information or Hint diagnostic is shown
  only when its origin is exact and its emitted span holds no construct's
  whole glue (`covers_construct_glue`: an `EmitAnchor` inside the span). A
  suggestion inside a construct's copied text (an unused name in an arm
  body) holds no whole anchor and is kept.

## Work log

- 2026-10-14: The 14 editor-matrix diffs of the killed nightly run were all
  the same added `tt only` TS80007 line; the same case fails on 0776bbb8.
  `git log -S relocated_origin` points at 0776bbb8 (TASK-795 Decision 5).
- 2026-10-14: Decision 1; `tests/cases/editor/aSuggestionOverAConstructsGlueIsNotShown.tt`.

## Issues and resolutions

### Issue 1: TS80007 over `await (match ...)` was shown

- **Symptom**: `tt only: main "await (match ...)" hint TS80007` in every
  `match_*_awaitOperand_*` matrix baseline.
- **Cause**: Decision 1's context.
- **Resolution**: Decision 1.

## Regression test (fails before the fix)

- **Path**: `tests/cases/editor/aSuggestionOverAConstructsGlueIsNotShown.tt`
- **Observed failure**: without the filter the baseline gained
  `4:10-4:79 "await (match (s) { ... })" hint TS80007`; the unused name in
  the arm body and the hint over a written `await n` are reported either way.

## Verification

- [x] `TT_MATRIX_CASES=all TT_CASES=match_blockArm_awaitOperand_optionalChain cargo test --release --test editor_cases`
- [x] `TT_CASES=aSuggestionOverAConstructsGlueIsNotShown cargo test --release --test editor_cases`
- [x] `cargo test` (run with the branch's other changes)
- [x] Baseline changes reviewed and committed with the change

## Result

`src/engine/language/project.rs`, `src/engine/language/service.rs`, the
editor case and its baseline.
