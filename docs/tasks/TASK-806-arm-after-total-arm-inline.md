# TASK-806: Keep an arm after one that always matches out of the conditional-expression form

- **Status**: Complete
- **Started**: 2026-10-14
- **Completed**: 2026-10-14
- **Commit**: `TASK-806: Keep an arm after one that always matches out of the conditional-expression form`

## Purpose

The full incremental sweep, rerun after TASK-801 and TASK-804, stopped 17
samples on an internal compiler error (`SourceOmitted`) that 0776bbb8 also
raises: `v = match (a, b) { (Up, Down) => 0, (_, _) => 1, _ => 2 };`.

## Scope

- Included: the plan's choice of the conditional-expression form for a
  match value (`can_defer_arm_values`).
- Excluded: reporting the unreachable arm; `match-wildcard-not-last`
  already covers `_` before another arm, and TypeScript reports the dead
  code of the statement form.

## Decisions

### Decision 1: Only the last arm may always match in a deferred value

- **Context**: A match value in an assignment or an argument is written as
  one conditional expression (`emit_inline_match`, or
  `emit_selected_arm_values` under conditional dispatch), which ends at the
  first arm that always matches; the arms after it kept only their
  comments, so their bodies (pass-through source) never reached the
  output, and `validate_source_preservation` stopped the build. The
  declaration form writes every arm.
- **Alternatives considered**: Exempting dropped arm bodies from the
  preservation check would silently delete user code and the type errors
  in it.
- **Decision and rationale**: `can_defer_arm_values` admits a decision only
  when no arm before the last always matches; any other decision takes the
  statement form, which writes every arm as the declaration form does.

## Work log

- 2026-10-14: Reduced the incremental ICE through the `arm_tag_unowned`
  probe text (an arm's tag replaced by `_`) to the assignment above; the
  CLI of 0776bbb8 raises the same ICE. Decision 1. The 17 samples pass.
- 2026-10-14: `tests/cases/conformance/evaluation/runtimeAnArmAfterAnArmThatAlwaysMatchesKeepsItsValueInAnAssignment.tt`.
- 2026-10-14: The first commit of this task carried a scratch probe test
  (`tests/zz_probe.rs`) by mistake and lacked this record; the next commit
  removes the probe and adds the record.

## Issues and resolutions

### Issue 1: SourceOmitted on a match value with an arm after a total arm

- **Symptom**: `validate_source_preservation broke the contract that every
  non-whitespace source byte outside tt-owned ranges reaches the target`.
- **Cause**: Decision 1's context.
- **Resolution**: Decision 1.

## Regression test (fails before the fix)

- **Path**: `tests/cases/conformance/evaluation/runtimeAnArmAfterAnArmThatAlwaysMatchesKeepsItsValueInAnAssignment.tt`
- **Observed failure**: `ttc` built from 0776bbb8 stopped with the
  `SourceOmitted` internal compiler error at bytes 232..233.

## Verification

- [x] The 17 incremental samples with `TT_INCREMENTAL_ONLY`
- [x] `TT_CASES=runtimeAnArmAfterAnArm… cargo test --release --test case_baselines`
- [x] `cargo test` (run with the branch's other changes)

## Result

`src/codegen/core/planning.rs`, the case and its baselines.
