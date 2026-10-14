# TASK-802: Regenerate the verify-failed matrix baseline TASK-764 left stale

- **Status**: Complete
- **Started**: 2026-10-14
- **Completed**: 2026-10-14
- **Commit**: `TASK-802: Regenerate the verify-failed matrix baseline TASK-764 left stale`

## Purpose

The nightly matrix step (`TT_MATRIX_CASES=all cargo test --release --test
case_baselines`) failed on
`tests/cases/conformance/diagnostics/verify-failed/verify-failed_missingExpression.tt`.
TASK-764 (0776bbb8) moved `use --no-verify to bypass` from the
`verify-failed` message to a `help:` line and regenerated the baselines a
pull request runs; this generated case runs only under
`TT_MATRIX_CASES=all`, so its baseline kept the old wording.

## Scope

- Included: the one `.errors.txt` baseline.
- Excluded: the rendering, which TASK-764 decided.

## Decisions

### Decision 1: Regenerate rather than change the message

- **Context**: The diff is exactly TASK-764's wording change; every other
  `verify-failed` baseline already carries the `help:` line.
- **Alternatives considered**: None; the case pins the current renderer.
- **Decision and rationale**: `TTC_REQUIRE_TSGO=1 TT_MATRIX_CASES=all
  UPDATE_EXPECT=1 TT_CASES=verify-failed_missingExpression cargo test
  --release --test case_baselines`, after reading the diff.

## Work log

- 2026-10-14: Regenerated and read the diff (message without the
  `; use --no-verify to bypass` suffix, plus `= help: use --no-verify to
  bypass` under the `--out-dir` report).

## Issues and resolutions

None.

## Regression test (fails before the fix)

Not applicable: no code changes; the baseline itself is the test.

## Verification

- [x] `TT_MATRIX_CASES=all TT_CASES=verify-failed_missingExpression cargo test --release --test case_baselines`
- [x] Baseline changes reviewed and committed with the change

## Result

`tests/baselines/reference/diagnostics/verify-failed/verify-failed_missingExpression.errors.txt`.
