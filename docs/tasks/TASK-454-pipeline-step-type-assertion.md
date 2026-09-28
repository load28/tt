# TASK-454: Apply a type assertion after a pipeline step to the whole pipeline

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

`const d = 1 |> String as string;` emitted `const d = String as string(1);`, which TypeScript rejects, and `satisfies` failed the same way. A step that was any other non-primary expression was also called without grouping: `1 |> f ?? g` emitted `f ?? g(1)`, valid TypeScript with a different meaning.

## Scope

- Included: Where a pipeline step ends (`src/parser/pipes.rs`), where the next pipeline's head starts after an assertion (`src/parser/parse.rs`), and how an inline step callee is grouped (`src/codegen/core/emitter/expression.rs`). The pipeline sections of `docs/ai/tt.md` and `docs/design/pipeline-operator.md`.
- Excluded: The head grammar, which already extends over `as` (`x as number |> f`). An `as` on the line after a pipeline: TypeScript does not continue an expression with a line-broken `as`/`satisfies` (TS1434 from the pinned `tsc` on `x\n  as string`), so `source-not-typescript` there is the correct report and is kept.

## Decisions

### Decision 1: `as`/`satisfies` after a step ends the step and asserts the pipeline value

- **Context**: The step scanner ran to the next `|>` or terminator and so took `String as string` as the step; codegen then appended the call to it. A choice was required between asserting the step's function and asserting the pipeline's value.
- **Alternatives considered**: (a) Keep `as T` inside the step and emit `(String as string)(1)`. That is well-formed but asserts the callee, which is almost never meant, and the step scanner would have to parse the type (`satisfies (n: number) => string` aborted the claim at `=>` as a stray pipe). (b) Make a step a unary-level expression, which would also move `x |> f ?? g` to `(x |> f) ?? g` and change the documented "step is an expression" grammar (design §3). (c) End the step only at a top-level `as`/`satisfies`.
- **Decision and rationale**: (c). The operand of `as`/`satisfies` is a type, never part of a step's function expression. TypeScript applies `as` to the whole operand on its left, and F# places its `:>` looser than `|>`, so `(1 |> String) as string` is the reading a reader expects. The head already extends over an assertion, so assertions and `|>` now group left to right at one level: after a pipeline ends at `as`/`satisfies`, the parser keeps the expression start, and a following `|>` takes `x |> f as string` as its head. `as`/`satisfies` counts only after a token that ends an expression (the parser's existing `ends_expression`), so a step naming a function `as` still pipes. To assert a step's function, parenthesize it.

### Decision 2: Group an inline step callee unless it is a primary expression

- **Context**: With an inert head, the step is emitted as the callee of a direct call. It was grouped only for a top-level comma.
- **Alternatives considered**: Rewriting such steps through `$tt_ap` would change the emitted shape of every inert pipeline.
- **Decision and rationale**: The callee position has the same rule as a receiver: a call binds as tightly as member access. The emitter now uses `push_receiver` there, so `f ?? g`, `await p`, and `new C` are called as a group, matching the `$tt_ap(x, f ?? g)` form a non-inert head already produced. Optional chains stay primary, so `x |> obj?.m` is still `obj?.m(x)`.

## Work log

- 2026-09-28: Reproduced: `1 |> String as string` and `satisfies string` emitted `String as string(1)`; `1 |> f ?? f` emitted `f ?? f(1)`; `1 |> f satisfies (n: number) => string` was a stray pipe. Confirmed with the pinned `tsc` that `x\n  as string` is TS1434.
- 2026-09-28: Added `asserts_pipeline`/`asserted` to `src/parser/pipes.rs` and exposed `ends_expression` from `src/parser/cursor.rs`; the step loop stops at a top-level assertion, and `parse.rs` keeps the head start after a pipeline that ends at one.
- 2026-09-28: Switched the inline step callee to `push_receiver` in `emit_apply`.
- 2026-09-28: Ran `TTC_REQUIRE_TSGO=1 cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `scripts/check-task-index`.
- 2026-09-28: Added output tests to `tests/compile/cases_11.rs` and a type-checked runtime test to `tests/integration.rs`; documented the grouping in `docs/ai/tt.md` and design §3.5.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: all suites passed, including the four new tests.
- [x] The runtime program of `runtime_a_type_assertion_after_a_pipeline_asserts_the_piped_value`, compiled with `ttc`, type-checked with the repository-pinned TypeScript 7.1 `tsc --strict`, and run with `node`, prints `1 4 4 1 14 5 6`.

## Result

Changed `src/parser/pipes.rs`, `src/parser/parse.rs`, `src/parser/cursor.rs`, `src/codegen/core/emitter/expression.rs`, `tests/compile/cases_11.rs`, `tests/integration.rs`, `docs/ai/tt.md`, `docs/design/pipeline-operator.md`, `docs/tasks/TASK-454-pipeline-step-type-assertion.md`, and `docs/tasks/INDEX.md`. A type assertion after a pipeline step now asserts the pipeline value, and a non-primary step is called as a group.
