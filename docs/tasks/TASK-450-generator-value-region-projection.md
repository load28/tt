# TASK-450: Project a value region in a generator as a generator so its `yield` parses

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

A `yield` in a match scrutinee or guard, such as
`match (yield 1) { A(n) => n, B => 0 }` in a generator, failed with a false
`source-not-typescript: Expression expected` at `yield`, and a `yield` that
crosses a `result` block reported `result-yield-crossing` followed by a
duplicate `source-not-typescript` at the same byte.

## Scope

- Included: The generator fact on Core IR decisions and Result regions
  (`src/core_ir/`), the tt-aware function target scan it uses
  (`src/flow/syntax.rs`), and the analysis projection's region function
  (`src/program_syntax/projection.rs`); `docs/ai/tt.md`.
- Excluded: Allowing `yield` in a block arm or across a `result` block; both
  remain tt errors (`match-control-crossing`, `result-yield-crossing`).

## Decisions

### Decision 1: The region function carries the enclosing generator kind

- **Context**: The analysis projection (the SWC view that lowering plans
  from; it is never emitted or type-checked) gives a value region statement
  positions with an immediately called `(async? () => { ... })()`. `await`
  was modelled by `is_async`, but an arrow can never contain `yield`, so any
  source `yield` inside the region could not parse. The emitted code hoists
  the decision into the generator's own statements and was already correct.
- **Alternatives considered**: (a) Add `ResultYieldCrossing` to
  `blocks_projection` so the second error is dropped. That hides the
  cascade but leaves scrutinee and guard `yield` broken. (b) Replace `yield`
  with a placeholder in the projection. That removes the suspension
  protocol frame lowering relies on to order operands around the `yield`.
  (c) Project a `function* () { ... }` when the region sits directly in a
  generator body.
- **Decision and rationale**: (c). `Decision` and `ResultRegion` gain
  `in_generator`, and the projection opens `(async? function* () { ... })()`
  for them. The projection is only parsed and walked, so the changed
  `this`/`arguments` binding of a function expression has no reader; SWC
  also accepts `super` property access there. A nested function written by
  the user still resets the fact, so a region inside a callback in a
  generator keeps the arrow and `yield` stays an error there as in
  TypeScript.

### Decision 2: Match syntax is not a function boundary for the generator fact

- **Context**: `function_target_at` reads tokens and treats every `=>` and
  every `) {` as a function. A match nested in another match's arm therefore
  looked like it sat in an ordinary arrow, kept the arrow projection, and
  still failed to parse (`A(n) => match ((yield n) as S) { ... }`).
- **Decision and rationale**: `user_function_target_at` takes the set of
  tt-owned tokens, as `user_function_depth_at` already does for the parser.
  Core IR lowering builds that set from HIR: each match's body brace (the
  first `{` after its head) and each arm's arrow (the first `=>` after the
  pattern or guard). `function_target_at` is that function with an empty
  set, so existing callers are unchanged.

## Work log

- 2026-09-27: Reproduced scrutinee, guard, cast-scrutinee, nested-arm,
  async generator, and `super` cases; confirmed `yield` in a pipe head,
  `if let`, and outside the match already worked and that the lowered
  output hoists the region into the generator.
- 2026-09-27: Added `in_generator` to Core IR, `user_function_target_at` to
  flow, the HIR-derived tt-owned token set to Core IR lowering, and the
  generator region function to the projection.
- 2026-09-27: Added a flow unit test, compile tests for the reported shapes
  and the single `result-yield-crossing`, and a runtime test that runs the
  generators under pinned `tsc --strict` and node.

## Issues and resolutions

### Issue 1: A match nested in an arm still failed to parse

- **Symptom**: After the first change, `A(n) => match ((yield n) as S) {...}`
  still reported `source-not-typescript` at the inner `yield`.
- **Cause**: The token-level function target read the outer arm's `=>` as a
  concise arrow function, so the inner region was not marked as in a
  generator.
- **Resolution**: Decision 2.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`

## Result

Changed `src/core_ir/mod.rs`, `src/core_ir/lower.rs`, `src/flow/mod.rs`,
`src/flow/syntax.rs`, `src/flow/tests.rs`, `src/program_syntax/projection.rs`,
`docs/ai/tt.md`, `tests/compile/cases_11.rs`, and
`tests/integration/cases_05.rs`. A `yield` in a match scrutinee, guard, or
expression arm in a generator compiles and runs; a `yield` crossing a
`result` block reports only `result-yield-crossing`.
