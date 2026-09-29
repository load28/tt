# TASK-550: Give a propagated operand its own storage

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: (see the work log)

## Purpose

A value-form `try` whose operand is a call around a `match` stored the
call's Result in the match's slot.
`const y = 1 + try r(match (v) { 1 => 1, _ => 2 }); return r(y - 1);`
emitted `let $tt_v1: number;`, the match writing `$tt_v1`, then
`$tt_v1 = $tt_v5($tt_v1);`, so `tsc` reported TS2322 three times and
TS18046. The same held for `[try r(match …)]` and
`console.log(try r(match …))`, in functions and in `result` blocks. Each
value needs storage of its own type.

## Scope

- Included: the storage a propagation reads its operand from
  (`emit_propagate_input`) and the slot a sequence is said to have
  (`structured_value_slot`).
- Excluded: the storage of other consumers, which already emit a sequence
  with values as an operand (`emit_nested_operand`).

## Decisions

### Decision 1: A sequence has the slot of the value it consists of, not of its last value

- **Context**: `structured_value_slot` answered a sequence with the slot of
  its last tt value (`body_tail_expr`), which `core_ir` documents as a
  scheduling anchor, "not necessarily the value of the entire enclosing
  host expression". `emit_propagate_input` then emitted the whole operand
  `r(match …)` into the match's slot, whose annotation is the match's type.
- **Alternatives considered**: (a) Allocate a slot for the operand in the
  plan. The propagation already reads its operand into a temporary of its
  own, so a slot would be a second copy. (b) Keep the tail rule and give
  the tail slot no annotation. The slot would still hold two different
  values.
- **Decision and rationale**: A sequence has a slot only when it consists
  of one value, apart from trivia and grouping parentheses
  (`grouped_sequence_value`, the same reading of parentheses
  `structured_grouping_frames` makes). Otherwise
  `emit_propagate_input` emits the operand as every other consumer does
  (`emit_nested_operand`): the values run into their own slots with the
  operand's captures scheduled first, and the operand is read once into the
  propagation's temporary, `const $tt_t0 = $tt_v2($tt_v1);`.

## Work log

- 2026-09-29: Reproduced with `ttc` and `tsc --strict` (TS2322, TS18046) in
  functions and in a `result` block.
- 2026-09-29: Changed `structured_value_slot`
  (`src/codegen/core/emitter/host.rs`), `emit_propagate_input`
  (`src/codegen/core/emitter/result.rs`), and made `emit_nested_operand`
  visible to it (`src/codegen/core/emitter/expression.rs`).
- 2026-09-29: Added
  `runtime_a_propagated_call_around_a_match_does_not_share_the_match_s_slot`
  (`tests/integration/cases_05.rs`).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] The new test fails without the change (TS2322, TS18046).

## Result

Changed `src/codegen/core/emitter/host.rs`,
`src/codegen/core/emitter/result.rs`,
`src/codegen/core/emitter/expression.rs`, `tests/integration/cases_05.rs`,
`docs/tasks/INDEX.md`, and this record.
