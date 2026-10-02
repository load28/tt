# TASK-601: Lower a value inside a for initializer's operands before the loop

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-601`

## Purpose

A value-form `try` in the first declarator of a multi-declarator `for`
head (`for (let x = try r(), i = 0; …)`) failed with `lowering-plan-failed`
("a `for` initializer assignment has no statement-safe rewrite"), reported
at the first tt construct of the file instead of at the `try` (TASK-593
noted it). The same area had two internal compiler errors: a value that is
an operand of a `for` initializer (`for (let x = g(match …); …)`,
`for (x = match …; …)`) reached emission with no plan ("match reached
expression emission without a host rewrite").

## Scope

- Included: the target plan for values in a `for` initializer
  (`src/codegen/core/planning.rs`), the Evaluation IR rejection that no
  longer applies (`src/evaluation_ir/evaluation.rs`, `src/evaluation_ir.rs`),
  the location of an Evaluation IR failure (`src/codegen/core/mod.rs`),
  `docs/ai/tt.md`, `docs/design/program-lowering.md` §7.9, and tests.
- Excluded: a value that reads a binding the head declares is TASK-600's
  placement rule, unchanged here.

## Decisions

### Decision 1: A value in a `for` initializer composes before the loop

- **Context**: A C-style `for` initializer is evaluated once, before the
  first test (ECMA-262 §14.7.4.2); an expression initializer declares
  nothing, and a declaration's first initializer reads no head binding
  outside a closure (TASK-600 rejects those). Moving its prelude before the
  loop is therefore faithful. The target plan wrote a prelude only for a
  value that is the whole initializer with no schedule
  (`OwnerSlotRewrite`); a value with steps (an argument, the right side of
  an assignment, a comma operand) had neither that plan nor the composed
  plan, which accepted only the `Compose` continuation. A value-form
  propagation there was rejected wholesale by the Evaluation IR
  (`UnsupportedForInitializer`), because that emission did not exist.
- **Alternatives considered**: (a) Reject these forms with a placement
  diagnostic. They have a faithful lowering. (b) Give values inside a `for`
  initializer the `Compose` continuation. The declaration-form `try`'s
  shadow projection relies on `ForInitialize` for its nested decision, and
  the change broke it. (c) Keep `ForInitialize` and let the composed plan
  take a `ForInitialize` value that has schedule steps, as it takes a
  `Compose` one.
- **Decision and rationale**: (c). The composed plan writes the prelude at
  the host owner (the `for` statement) and delivers the value where it was
  written, which is what an operand needs. With that emission in place the
  `UnsupportedForInitializer` rejection has no case left, and it is
  removed: `for (x = try r(); …)` and `for (let x = try r(), i = 0; …)`
  now lower like the declaration form already did.

### Decision 2: An Evaluation IR failure is reported where it happened

- **Context**: `LoweringFailure::Evaluation` took the file's first tt node
  as its location even when the error carried its own span.
- **Decision and rationale**: `EvaluationError::source` answers the span a
  variant carries (`DiscardedResult`, `RepeatedPropagation`,
  `InvalidHostOwner`), and the lowering uses it, falling back to the
  primary source only for errors without one.

## Work log

- 2026-09-30: Reproduced the `lowering-plan-failed` at 1:9 and the two
  internal compiler errors.
- 2026-09-30: `src/codegen/core/planning.rs` (the composed plan's
  continuation filter), `src/evaluation_ir/evaluation.rs` and
  `src/evaluation_ir.rs` (the rejection and its variant removed,
  `EvaluationError::source`), `src/codegen/core/mod.rs`.
- 2026-09-30: A first attempt classified only a whole declarator
  initializer as `ForInitialize`; it broke the declaration-form `try` with
  a nested match (`every_value_region_crosses_every_host_protocol_class`)
  and was replaced by Decision 1 (c).
- 2026-09-30: Tests: `try_assignment_in_for_initializer_runs_before_the_loop`
  (was `…_reports_a_located_lowering_diagnostic`,
  `tests/compile/cases_04.rs`),
  `a_value_nested_in_a_for_head_initializer_runs_before_the_loop`
  (`tests/compile/cases_14.rs`), and
  `runtime_a_for_head_initializer_value_runs_before_the_loop_when_faithful`
  (`tests/integration/cases_05.rs`, tsc + node).

## Issues and resolutions

### Issue 1: Whole-initializer classification broke the declaration-form `try`

- **Symptom**: `for (let x = try (match (flag) { … });;)` emitted
  TypeScript that did not parse.
- **Cause**: The nested decision of a declaration-form propagation is
  exposed through a projection-only comma, and its plan depends on the
  `ForInitialize` continuation.
- **Resolution**: Kept the continuation; changed the plan instead
  (Decision 1).

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 cargo test`
- [x] `node scripts/check-task-index`

## Result

Changed `src/codegen/core/planning.rs`, `src/codegen/core/mod.rs`,
`src/evaluation_ir.rs`, `src/evaluation_ir/evaluation.rs`, `docs/ai/tt.md`,
`docs/design/program-lowering.md`, `tests/compile/cases_04.rs`,
`tests/compile/cases_14.rs`, `tests/integration/cases_05.rs`,
`docs/tasks/INDEX.md`, this record, and a note on TASK-593.
