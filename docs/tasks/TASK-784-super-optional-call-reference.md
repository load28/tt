# TASK-784: Fix two regressions of TASK-781 found by the seventh audit

- **Status**: Complete
- **Started**: 2026-10-07
- **Completed**: 2026-10-07
- **Commit**: `TASK-784: Fix two regressions of TASK-781 found by the seventh audit`

## Purpose

The seventh audit of the compiler found two regressions of TASK-781:
`super.m?.(v)` with a lowered argument read the method as `this.m`
(decision K7), and a match whose arm values are selected in place, as the
discarded left operand of a comma, was fused into the next operand
(decision K3).

## Scope

- Included: the two regressions.
- Excluded: the rest of the seventh audit's compiler findings.

## Decisions

### Decision 1: A `super` receiver is `super` in the reference and `this` as the call's receiver

- **Context**: The emitter writes a planned receiver in two roles: inside
  the member reference it reads (`mapped`), and as the `this` argument of
  `.call(...)` or `.bind(...)`. ECMA-262 `MakeSuperPropertyReference`
  looks the property up on the home object's prototype and calls it with
  the environment's `this`.
- **Decision and rationale**: In the reference the receiver is written as
  the source wrote it (`super`), and as the call's receiver it is `this`.

### Decision 2: A discarded comma operand is never written in the selected-arm form

- **Context**: `(match (o()) { A => typeof 2, B => "b" }, (x * 2))` emitted
  `(($tt_v0 === 0 ? typeof 2 : "b") (x * 2))`, a call, and `(match ...,
  x * 2)` failed to parse: K3 removes a discarded operand's slot read and
  its comma, but a value in the selected-arm form (its arm values written in
  place behind a selector) has no slot read to remove.
- **Decision and rationale**: A value that is a discarded comma operand is
  planned with its arms writing its slot in the prelude, as every other
  discarded value is, so K3's removal applies; the arms still run in order
  for their effects.

## Work log

- 2026-10-07: Fixed `push_planned_receiver` in
  `src/codegen/core/emitter/host.rs`; extended
  `aSuperMethodBehindATypeWrapperOrOptionalCallKeepsThis` with an
  overriding method that calls `super.m?.(...)`.
- 2026-10-07: Fixed the discarded selected-arm value in
  `src/codegen/core/planning.rs`, with
  `aSelectedArmValueAsADiscardedCommaOperandLeavesNothingBehind`.

## Issues and resolutions

None.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/aSuperMethodBehindATypeWrapperOrOptionalCallKeepsThis.tt`
- **Observed failure**: on `a71c5729` the program exited 1 with
  `RangeError: Maximum call stack size exceeded` (the capture read `this.m`).
- **Path**: `tests/cases/compiler/aSelectedArmValueAsADiscardedCommaOperandLeavesNothingBehind.tt`
- **Observed failure**: on `a71c5729`, `error[verify-failed]: generated
  TypeScript failed to parse: Expected ',', got 'ident'`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] Baseline changes reviewed and committed with the change

## Result

Complete. Both regressions are fixed with cases that fail on `a71c5729`, and the full gate passes.
