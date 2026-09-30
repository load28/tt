# TASK-571: Lower every value a `result` return's argument consumes in the return's prelude

- **Status**: Complete
- **Started**: 2026-09-29
- **Completed**: 2026-09-29
- **Commit**: (see the work log)

## Purpose

A `return` inside a `result` block whose argument held a `try` next to any
other tt value produced invalid output.
`return result { return (try a()) + (try a()); };` failed the output
self-check (`verify-failed`); with `--no-verify` the second `try` was left
as written, a stray `$tt_v2` followed the `break`, and the block's storage
annotation contained `try: TResult<…>`. The same held for
`return [try a(), try b()]`, `return { x: try a(), y: try b() }`,
`return f(try a(), try b())`, `return match (k) {…} + (try a())`,
`return (a() |> f) + (try a())`, and the same returns under an `if`.
`docs/ai/tt.md` says sibling `try`s propagate left to right and
`return x` completes the block with `Ok(x)`.

## Scope

- Included: which values of a return argument transfer to the exit of the
  region the return leaves (Evaluation IR builder), the emitter's reading
  of that decision for a propagating return, updated output assertions,
  and runtime and `--check-types` regression tests.
- Excluded: a `try` in a match block arm's return and in a returned
  template inside a `result` block, which sema reports as documented
  placement errors (`try-placement`, `try-crosses-value-region`).

## Decisions

### Decision 1: A value transfers to a region exit only when the return delivers it as it is

- **Context**: TASK-549 (Decision 4) made a `match` inside a return
  argument a host value of the return statement unless the argument
  delivers it as it is, but kept every other value kind (`try`, a
  pipeline, a nested `result`) nested in the region's exit whenever it lay
  inside the argument. The exit emitter lowers one propagation per return
  by replacing its span in the argument text (`result_return_propagate`),
  so a second value in the same argument was never lowered, and a `match`
  planned as a host value next to a `try` planned into the exit emitted
  two conflicting lowerings of one statement. The rule also read "as it
  is" off the protocol steps, and a value in a conditional's test or a
  logical operator's left operand has no step, so
  `return (try a()) ? (try b()) : 0` still transferred its test to the
  exit.
- **Alternatives considered**: (a) Teach the exit emitters to lower an
  argument with several values. That is a second planning path for values
  the return statement's owner already plans, which TASK-549 rejected.
  (b) Apply TASK-549's rule to every value kind and decide "as it is"
  structurally.
- **Decision and rationale**: (b). A value transfers to the exit when it
  is the return's argument under parentheses and TypeScript wrappers
  (`HostExit::value_argument`, read off the AST) or an interpolation of a
  returned template, which the exit delivers itself (`delivered_by_exit`,
  `src/evaluation_ir/builder.rs`). Every other value is an ordinary value
  of the return statement, planned with its protocol: siblings run left to
  right, a callee is captured first (ECMA-262 `EvaluateCall`), a
  conditional branch is a conditional operation, and the return writes
  `Ok(<argument with slots>)` through the generic exit edit.

### Decision 2: The emitter asks the plan which propagation the exit owns

- **Context**: `result_return_propagate` found the first `try` anywhere in
  the argument, independent of where the plan placed it.
- **Decision and rationale**: It accepts only a propagation the plan placed
  in the exit (`structurally_nested_values`), so the plan is the single
  source of that decision for both the exit emitter and the source walk.
- **Consequence**: `return Math.round(try total() * 1.1)` in a `result`
  block now captures `Math.round` before `total()` runs, as the same
  return in a function already did; the propagation's value is read into
  its slot. Two output assertions were updated.

## Work log

- 2026-09-29: Reproduced `target/probe4-compiler/min/b1.tt` and the listed
  shapes with `ttc -p` and `ttc --check-types`.
- 2026-09-29: Changed `placement` and `delivered_by_exit`
  (`src/evaluation_ir/builder.rs`) and `result_return_propagate`
  (`src/codegen/core/emitter/result.rs`).
- 2026-09-29: Updated `result_region_composes_embedded_try_and_pipeline_try`
  (`tests/compile/cases_01.rs`) and
  `result_return_expression_propagates_to_the_result_scope`
  (`tests/compile/cases_07.rs`) for Decision 2's consequence.
- 2026-09-29: Added
  `runtime_several_values_in_a_result_return_argument_run_in_the_return_s_prelude`
  (`tests/integration/cases_05.rs`) and
  `sibling_tries_in_a_result_return_check_clean`
  (`tests/native/cases_10.rs`).

## Issues and resolutions

### Issue 1: A conditional's test still transferred to the exit

- **Symptom**: With the TASK-549 rule applied to every value kind,
  `return (try a()) ? (try b()) : 0` emitted `const $tt_t0 = ;` and read an
  unassigned capture of the test.
- **Cause**: The test position has no protocol step, so a step-based "as it
  is" check accepted it.
- **Resolution**: Decision 1 compares the value with the return's AST
  argument.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] Both new tests fail without the change (`tsc` rejects the runtime
  program; `--check-types` reports `verify-failed`).

## Result

Changed `src/evaluation_ir/builder.rs`,
`src/codegen/core/emitter/result.rs`, `tests/compile/cases_01.rs`,
`tests/compile/cases_07.rs`, `tests/integration/cases_05.rs`,
`tests/native/cases_10.rs`, `docs/tasks/INDEX.md`, and this record.
