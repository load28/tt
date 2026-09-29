# TASK-457: Keep a conditional value's operand when a later sibling captures it

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: see `git log --grep TASK-457`

## Purpose

Two conditional value-form `try`s in one expression emitted the first
operand as nothing: `[c ? try p(s) : 0, c ? try p(s) : 1]` produced
`const $tt_t0 = ;` and failed verification. The same happened for
`(n && try p(s)) + (n && try p(s))`, `f(n && try p(s), n && try p(s))`, and a
template literal with two such interpolations. A single conditional `try`
compiled correctly.

## Scope

- Included: the emitter's conditional-operation lowering
  (`src/codegen/core/emitter/host.rs`); compile and runtime tests.
- Excluded: nested conditionals (`c ? (d ? try p(s) : 1) : 0`), which are
  already rejected with `try-placement` even alone.

## Decisions

### Decision 1: A conditional operation's active value is an active structured value while its region is emitted

- **Context**: The second conditional operation needs every earlier sibling
  evaluated first, so the plan captures the first element's source
  (`c ? try p(s) : 0`) into a slot (`const $tt_v4 = ($tt_v5);`) and records
  a source replacement for that span. The first operation's region is
  emitted before that capture. While it emitted the `try` operand `p(s)`
  through the source walk, the walk found the unanchored replacement for the
  enclosing element, which contains the cursor, and skipped to its end
  without writing anything, because the value being lowered was not marked
  active. A compose `Value` action marks its value with
  `active_structured_exprs.enter` for exactly this reason
  (`replacement_contains_active_value` then suppresses every replacement that
  contains it); the `Operation` action path did not.
- **Alternatives considered**: (a) excluding captures that start before the
  emitted span from the source walk — changes the rule for every source
  range, including ones that legitimately read a captured prefix; (b)
  reordering planning so no capture covers an earlier conditional — the
  capture is required to keep left-to-right evaluation.
- **Decision and rationale**: `emit_conditional_active_branch` enters the
  branch value as active for the whole emission of its region, the same
  contract a compose `Value` action already follows, so every conditional
  kind (ternary, `&&`, `||`, `??`, optional call) gets it from one place.

## Work log

- 2026-09-28: Reproduced the four reported shapes with `ttc -p --no-verify`;
  narrowed it to a later sibling that captures the earlier conditional
  element (`[c ? try p(s) : 0, try p(s)]` was already correct).
- 2026-09-28: `src/codegen/core/emitter/host.rs`: enter the active value in
  `emit_conditional_active_branch`.
- 2026-09-28: Tests
  `a_conditional_try_keeps_its_operand_when_a_later_sibling_captures_it`
  (`tests/compile/cases_11.rs`; fails without the fix) and
  `runtime_sibling_conditional_trys_each_evaluate_their_operand_in_order`
  (`tests/integration/cases_05.rs`, tsc + node).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] Runtime: each operand runs once, left to right, and an `Err` in the
  first stops before the second (`ok 12 [1,2] err bad e [e] ok 1 []`,
  `ok 7 [3,4] err bad e [3,e] ok 0 []`, `ok 56 [5,6] err bad e [e]`,
  `ok 7-8 [7,8] ok 0-1 []`).

## Result

Changed `src/codegen/core/emitter/host.rs`, `tests/compile/cases_11.rs`,
`tests/integration/cases_05.rs`, `docs/tasks/INDEX.md`, and this record.
