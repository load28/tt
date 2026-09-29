# TASK-522: Evaluate an assignment's target before a hoisted right operand

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: (see the work log)

## Purpose

A value-form `try`, a `match`, or a pipeline on the right of an assignment is
hoisted in front of its statement, and it ran before the assignment's target
was evaluated. `n += try (n = 100, ok(5))` left `n` at 105, where JavaScript
gives 6; `target().v = try step()` called `step` before `target`, and never
called `target` when `step` failed; `m += match (m) { 1 => { m = 100; return 5; }, _ => 0 }`
gave 105 instead of 6. `docs/ai/tt.md` promises left-to-right evaluation,
and `x = x + (try ...)` already captured its left operand.

## Scope

- Included: the evaluation protocol of an assignment's target — a member
  target's object and computed key, and a compound operator's read of the
  target's current value — in the syntax layer, the schedule, and the target
  emission.
- Excluded: the conditional evaluation of a logical assignment's right
  operand (`&&=`, `||=`, `??=`), which is still hoisted unconditionally (see
  Issue 2 and the follow-up suggested at the end of the session).

## Decisions

### Decision 1: An assignment is its own protocol frame, not an ordered list with one position

- **Context**: `visit_assign_expr` pushed an `Ordered` frame whose only
  position was the right operand, so no step of a value on the right ever
  had an input from the target. ECMA-262 §13.15.2 evaluates
  `LeftHandSideExpression` to a Reference Record first, then (for a
  compound operator) `GetValue`s it, then evaluates the right operand, and
  finally `PutValue`s the result. `docs/design/program-lowering.md` §5–§6
  names the assignment target as a value kind of its own.
- **Decision and rationale**: `ProjectedProtocolFrame::Assignment` records
  the operator, the target's span, and the target's reference parts (a
  member's object, then its computed key, looking through parentheses and
  TypeScript's `as`, `satisfies`, `!`, `<T>`, and instantiation wrappers).
  The step of a value on the right carries those parts as ordinary value
  inputs, so the existing capture machinery evaluates them first. A
  Reference Record holds the key's value, and `ToPropertyKey` runs in
  `PutValue`, so capturing the key's value and writing `$o[$k] = ...` after
  the right operand converts it at the same point a native assignment does.
  A destructuring target is evaluated after the right operand and has no
  parts.

### Decision 2: A compound operator reads the target into an accumulator and keeps its authored target

- **Context**: The read of the current value has to precede the right
  operand, and the result has to be written through the target. Rewriting
  `t += v` as `t = $old + (v)` needs parentheses around an arbitrary right
  operand, which the source walk (text replacements of source spans) cannot
  close.
- **Alternatives considered**: (a) Lower a compound assignment as a whole
  operation that re-emits target and right operand (the conditional
  operation machinery). It replaces the whole assignment by a result slot,
  and a whole operation cannot nest inside another one, so
  `c && (n += try f())`, which compiles today, would become a placement
  error. (b) Read into an accumulator and rewrite only the operator:
  `t = $acc += v`.
- **Decision and rationale**: (b). `let $acc = (t)` is a capture
  (`EvaluationInputMode::CompoundAssignmentTarget`) that reads the target
  through the slots of the object and key captured before it. The operator
  token becomes `= $acc +=`: the accumulator applies the operator to the
  value read before the right operand, and the assignment writes the result
  through the target, so no parentheses are needed and the step stays an
  eager step that composes with every other protocol. The target is printed
  twice and its operator rewritten, so the plan records both as rewritten
  source and the target as relocated. A capture may now contain an earlier
  capture of the same schedule (`target_capability`); `validate_order`
  already accepts that as a dependency, and `captured_range` reads the
  earlier slot. `SourceReplacement::rewrite` carries the operator text, and
  every place that prints a replacement prints `written()`.

### Decision 3: An identifier or `this` in the target is read again at the assignment

- **Context**: Capturing every object broke TypeScript on valid programs.
  `this.outcome = result { ... }` in a constructor lost the class property
  TypeScript infers from it (`runtime_nested_results_preserve_constructor_and_generator_protocols`
  failed with TS7008), and `state.value = try read(); state.value.toFixed()`
  failed with TS18048, because TypeScript narrows an assigned reference only
  when the assignment names it.
- **Alternatives considered**: (a) Capture every object and key. Exact for a
  right operand that rebinds the identifier, but valid programs stop
  type-checking. (b) Classify `this` as inert in `expression_effects`. That
  changes every other capture of `this`, such as a method receiver. (c)
  Re-read an identifier or `this` part at the assignment, as TypeScript's
  own down-level transforms do for a simple-copiable operand
  (`isSimpleCopiableExpression`, used by `transformLogicalAssignment` for
  the object and the key).
- **Decision and rationale**: (c), in the syntax layer
  (`assignment_reference`). `this` cannot change once bound. An identifier
  names the same object at the write unless the right operand rebinds it,
  which is the one case where the write does not reach the object the
  reference named first; `docs/design/program-lowering.md` §7.6 and
  `docs/ai/tt.md` say so. Every listed failure has a call, a read, or a
  compound operator in its target and is fixed.

## Work log

- 2026-09-29: Reproduced the three cases from the probe
  (`target/probe-compiler/p1/src/order.tt`, `e2.tt`, `e6.tt`).
- 2026-09-29: Added the assignment frame and its step inputs
  (`src/program_syntax/collector.rs`, `visit.rs`, `protocol.rs`), the
  accumulator mode (`src/program_syntax.rs`), the nested-capture rule
  (`src/evaluation_ir/planning.rs`), and the operator rewrite, accumulator
  capture, and source-preservation claims (`src/codegen/core/planning.rs`,
  `src/codegen/core/emitter/host.rs`, `source.rs`).
- 2026-09-29: Probed member, private, `super`, computed, parenthesized,
  `as`, `!`, string, `**=`, expression-position, conditional-branch, owned
  child (`match` arm), template, array, and loop-body assignments against
  Node's results; all agree.
- 2026-09-29: Tried lowering logical assignments as whole conditional
  operations. The runtime order was right, but capturing the condition lost
  TypeScript's narrowing of the target (`u ??= try f(); use(u)` failed with
  TS2322), and keeping it requires a lowering that reads the target once and
  still writes its value only when the result is consumed. Reverted
  (Issue 2).
- 2026-09-29: Added `runtime_assignment_evaluates_its_target_before_a_hoisted_right_operand`
  and `an_assignment_target_keeps_the_narrowing_typescript_gives_it`
  (`tests/integration/cases_05.rs`) and
  `an_assignment_captures_its_target_before_a_hoisted_right_operand`
  (`tests/compile/cases_14.rs`). Updated `docs/ai/tt.md` and
  `docs/design/program-lowering.md` §7.6.

## Issues and resolutions

### Issue 1: Capturing `this` broke a constructor's property inference

- **Symptom**: Two integration tests failed with TS7008 (`Member 'outcome'
  implicitly has an 'any' type`).
- **Cause**: TypeScript infers an unannotated property from
  `this.outcome = ...` in a constructor; `$tt_v1.outcome = ...` is not that
  form.
- **Resolution**: Decision 3.

### Issue 2: A logical assignment still evaluates its right operand unconditionally

- **Symptom**: `v ||= try ok(5)` runs `ok(5)` although `v` is truthy.
- **Cause**: The frame treats every operator's right operand as eager. The
  right operand of a logical assignment is conditional, like `v || ...`,
  and needs a whole-operation lowering that tests the target's current value
  once, writes only on the taken path, and keeps TypeScript's narrowing of
  the target.
- **Resolution**: Out of this task's scope; the reference parts are captured
  in order, and the conditional lowering is left for a follow-up.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`

## Result

Changed `src/program_syntax.rs`, `src/program_syntax/collector.rs`,
`src/program_syntax/protocol.rs`, `src/program_syntax/visit.rs`,
`src/evaluation_ir/planning.rs`, `src/codegen/core/planning.rs`,
`src/codegen/core/emitter/host.rs`, `src/codegen/core/emitter/source.rs`,
`docs/ai/tt.md`, `docs/design/program-lowering.md`,
`tests/integration/cases_05.rs`, and `tests/compile/cases_14.rs`. An
assignment's object, computed key, and (for a compound operator) current
value are evaluated before a hoisted right operand.
