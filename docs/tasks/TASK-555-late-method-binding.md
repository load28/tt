# TASK-555: Bind a captured method at its call, after the arguments

> Superseded in part by TASK-573: a method is no longer captured and bound;
> the call reads it through its receiver where the call is made, after the
> arguments (Decision 1 below no longer describes the lowering).

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: (see the work log)

## Purpose

When a call's argument holds a tt value, the lowering captured the method
as `const $f = ($r.m).bind($r);` before the arguments ran, so
`o.missing(match (m()) {…})` threw a `TypeError` from `bind` before `m()`
ran. ECMA-262 `EvaluateCall` reads the member (`GetValue`), evaluates the
arguments (`ArgumentListEvaluation`), and only then throws when the value
is not callable (`IsCallable`).

## Scope

- Included: the capture of a member callee (`emit_scheduled_step`), every
  place a call reads that capture (the source walk's replacements, operand
  and nested-capture readings, call completions), the callee span of a
  member call (`visit_call_expr`), updated output assertions, the
  `contextual-consumed-call` emit fixture, and a regression test.
- Excluded: optional calls, which TASK-545 already calls through the
  receiver after the test; trailing arguments written after the tt value
  in the same call are still evaluated after `bind` (a non-callable member
  throws before them), since keeping them in the call keeps their
  contextual types.

## Decisions

### Decision 1: Capture the method, bind it where the call is made

- **Context**: The capture has to read the member before the arguments (a
  getter runs there), and the call needs the receiver as `this`.
- **Alternatives considered**: `$f.call($r, …)` at the call. Checked with
  `tsc --strict`: `CallableFunction.call` instantiates a generic method's
  type parameters with `unknown` (`const n: number = f.call(r, 10)` is
  TS2322), where `bind` keeps the signature.
- **Decision and rationale**: The capture is `const $f = ($r.m);`, and
  every reading of it as a callee is `$f.bind($r)` (`bound_callee` in
  planning, `captured_reading` in the emitter), so `bind` runs at the call,
  after the arguments the prelude evaluated.

### Decision 2: A method is captured with its TypeScript wrappers

- **Context**: `o.missing!(match …)` captured `$r.missing` without the `!`,
  so the capture was `F | undefined` (TS2532 before this task, TS18048 at
  `$f.bind($r)` after Decision 1).
- **Decision and rationale**: For a member callee the call frame's callee
  span is the whole callee expression (`o.missing!`, `(o.m as F)`), so the
  capture keeps the author's assertion and the call binds that type.

## Work log

- 2026-09-29: Reproduced with `ttc` and node: the `TypeError` came before
  `m()` ran.
- 2026-09-29: Changed `src/codegen/core/planning.rs` (`bound_callee`,
  `optional_call_step`, the member-reference replacement, the completion's
  invoke), `src/codegen/core/emitter/host.rs` (the capture,
  `captured_reading`), and `src/program_syntax/visit.rs` (the callee span).
- 2026-09-29: The first version lost `this` in a postfix pipeline step
  (`g() |> .m(match …)` wrote `$f($v)`), since operand emission read the
  slot directly; `captured_reading` now serves every reading. Updated the
  assertions of `an_inert_member_receiver_needs_no_receiver_slot`,
  `a_script_statement_that_declares_no_lexical_global_is_enclosed_with_its_storage`,
  and `a_member_step_captures_its_method_from_the_piped_value_before_the_argument`,
  and the `contextual-consumed-call` fixture (`UPDATE_EXPECT=1`; the diff
  moves `.bind` from the capture to each arm's call).
- 2026-09-29: Added
  `runtime_a_missing_method_throws_after_the_call_s_arguments`
  (`tests/integration/cases_05.rs`).

## Issues and resolutions

### Issue 1: A postfix pipeline step lost its receiver

- **Symptom**: `$tt_v0 = $tt_v2($tt_v1);` with `$tt_v2` an unbound method.
- **Cause**: `source_range_with_scheduled_values` and `captured_range`
  wrote the slot name of a nested schedule's input directly.
- **Resolution**: Both read the input through `captured_reading`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] The new test fails without the change (TS2532; at runtime the
  `TypeError` came before `m()`).

## Result

Changed `src/codegen/core/planning.rs`, `src/codegen/core/emitter/host.rs`,
`src/program_syntax/visit.rs`, `tests/compile/cases_05.rs`,
`tests/compile/cases_11.rs`, `tests/compile/cases_14.rs`,
`tests/fixtures/emit/contextual-consumed-call/expected.ts`,
`tests/integration/cases_05.rs`, `docs/tasks/INDEX.md`, and this record.
