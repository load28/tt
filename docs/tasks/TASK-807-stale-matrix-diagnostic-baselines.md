# TASK-807: Regenerate the matrix diagnostic baselines left stale since TASK-514

- **Status**: Complete
- **Started**: 2026-10-14
- **Completed**: 2026-10-14
- **Commit**: `TASK-807: Regenerate the matrix diagnostic baselines left stale since TASK-514`

## Purpose

The nightly matrix step (`TT_MATRIX_CASES=all cargo test --release --test
case_baselines`) failed on 61 generated cases besides TASK-802's one; an
earlier read of the same run's log stopped at its last lines and missed
them. Their baselines were last written by 67c98aa8 (TASK-514), and
0776bbb8 produces exactly this branch's new output, so they are stale, not
changed by this branch. These cases run only under `TT_MATRIX_CASES=all`.

## Scope

- Included: 45 `match-or-binding-mismatch_differentNames_*`, one
  `variant-field-shadows-tag_kindField`, and 15 `variant_fieldShadowsTag_*`
  `.errors.txt` baselines.
- Excluded: renderer and mapping code.

## Decisions

### Decision 1: Regenerate after reading every change

- **Context**: The diff has two classes and nothing else:
  1. `match-or-binding-mismatch`: the `ts2339` for the binding `r` that
     only one alternative binds is now placed on `r` itself
     (`main.tt:5:62`, "Property 'r' does not exist on type ...") instead of
     on the scrutinee with the "needs a value with a `kind` discriminant —
     this scrutinee has none" explanation, which was wrong for a variant
     scrutinee. The explanation (`engine/semantics/translate.rs`) applies
     only to an error in the scrutinee's glue; the error now maps exactly
     onto the binding, through the mapping work merged in 0776bbb8.
  2. `variant-field-shadows-tag` / `variant_fieldShadowsTag_*`: the
     `ts2717` and `ts2322` reported at the same place appear in the other
     order; repeated runs give the same order.
- **Decision and rationale**: `UPDATE_EXPECT=1 TTC_REQUIRE_TSGO=1
  TT_MATRIX_CASES=all TT_CASES=<filter>` for `variant_fieldShadowsTag`,
  `match-or-binding-mismatch`, and `variant-field-shadows-tag_kindField`.

## Work log

- 2026-10-14: Found the 61 in the rerun of the matrix step; reran the
  filters twice (identical output) and on a 0776bbb8 worktree (identical
  output); reviewed and regenerated.

## Issues and resolutions

### Issue 1: The first reading of the matrix log missed 61 failures

- **Symptom**: TASK-802 regenerated one baseline; the step still failed.
- **Cause**: Only the log's last lines were read.
- **Resolution**: Counted every `modified baseline` line of the log.

## Regression test (fails before the fix)

Not applicable: no code changes; the baselines are the tests.

## Verification

- [x] The three filters pass with `TT_MATRIX_CASES=all`
- [x] Baseline changes reviewed and committed with the change

## Result

61 baselines under `tests/baselines/reference/diagnostics/` and
`tests/baselines/reference/matrix/variant/`.
