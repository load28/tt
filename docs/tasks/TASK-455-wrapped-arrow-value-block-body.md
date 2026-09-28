# TASK-455: Lower a wrapped concise-arrow value to a block body

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

A concise arrow body whose `match` or `result` value was followed by `as T` or `satisfies T`, or wrapped in parentheses, lowered to a non-async IIFE: `async (p: Promise<V>) => match (await p) { ... } as number` failed the output self-check with `await isn't allowed in non-async function`, and the non-async form emitted `(() => {...})() as number`, which contradicts the `docs/ai/tt.md` rule that a match never emits an IIFE. The same body with `+ 1` already became a block body.

## Scope

- Included: The host continuation of a value in a concise arrow body (`src/program_syntax.rs`, `src/program_syntax/visit.rs`) and the arrow-return rewrite that no longer needs an IIFE form (`src/codegen/core/planning.rs`, `src/codegen/core/emitter/host.rs`).
- Excluded: `return` statements and declaration initializers, whose owner is the statement and which already keep a wrapper such as `as T` around the slot.

## Decisions

### Decision 1: Only a value whose source is the whole arrow body has the arrow-return continuation

- **Context**: `host_continuation` skipped transparent edges (parentheses, `as`, `satisfies`, `!`, type assertions, instantiation) before classifying the parent, so a wrapped value had the `ArrowReturn` continuation. The arrow-return rewrite replaces only the value, so when the value was smaller than the body it had to stay an expression and emitted an IIFE (`parenthesized: rewrite.owner.span != value.source`).
- **Alternatives considered**: (a) Make the IIFE `async` when the arrow is. That keeps an IIFE the language rules out, and an async IIFE would also turn `yield` in an enclosing generator into an error and add a promise the arrow never awaited in its own body. (b) Teach the arrow-return rewrite to re-emit the wrapper around the slot. That duplicates what the compose path already does for `+ 1`, which rewrites the body into `{ ...; return <body with the value replaced>; }`. (c) Decide from the AST path alone, allowing only plain `Expr` edges between the value and the arrow body. The projection wraps every expression placeholder in its own parentheses, so the path cannot tell those from authored ones; this sent `value |> (x => try next())` to the compose path and broke two existing tests (see Issue 1).
- **Decision and rationale**: The overlay builder already knows both the value's source span and its host owner's source span. It now passes `value_is_owner` (the two spans are equal) into `EvaluationContext::from_path`, and an `ArrowReturn` continuation whose value is not the whole body becomes `Compose`, the path the `+ 1` case already takes. This is the same span fact the planner used to choose the IIFE, now decided once where the continuation is. The emitted body is `{ let slot: T; ...; return slot as number; }`, the `await` stays in the async arrow, and the value slot is still typed. With the continuation guaranteeing that the value is the body, the rewrite's IIFE branch had no remaining input, so the `parenthesized` field and branch were removed.

## Work log

- 2026-09-28: Reproduced the `await` self-check failure and the `(() => {...})() as number` output for `as`, `satisfies`, `(match ...) as number`, and plain `(match ...)`, and the same for `async (n) => result { ... } as R`.
- 2026-09-28: Traced the continuation to `host_continuation` and the IIFE to `emit_arrow_return_rewrite`. First restricted `ArrowReturn` in `host_continuation` to paths with only `Expr` edges; the compile suite failed (Issue 1).
- 2026-09-28: Moved the decision to the overlay builder: `OverlayFacts::value_is_owner` compares the value's source span with its host owner's span, and `from_path` turns a partial-body `ArrowReturn` into `Compose`. Removed `ArrowReturnRewrite::parenthesized` and the IIFE delimiters.
- 2026-09-28: Added a program-syntax unit test for the continuation of wrapped bodies, an output test in `tests/compile/cases_11.rs`, and a type-checked runtime test in `tests/integration.rs`. Clarified the rule in the `docs/ai/tt.md` match section.
- 2026-09-28: Added an output test for a parenthesized value inside a parenthesized pipeline step arrow (`value |> (x => (try next()))`), which stays inside that arrow's new block body. Ran `TTC_REQUIRE_TSGO=1 cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `scripts/check-task-index`.

## Issues and resolutions

### Issue 1: A path-only rule broke a `try` in a parenthesized pipeline arrow

- **Symptom**: `pipeline_concise_arrow_keeps_try_in_the_arrow` and `placement_matrix_prerequisite_gate` failed with `generated TypeScript failed to parse: unbalanced TypeScript delimiter`; `value |> (x => try next())` emitted `({ ...; return (x => $tt_v0; })`.
- **Cause**: Printing the paths showed `[ArrowFunctionBody(Expr), Expr(Paren), ParenExpr(Expr), Expr(Ident)]` for every `try` value: the projection writes each expression placeholder as `($tt_syntax_expr_N)`, so its own `ParenExpr` edge is on every path and cannot be told from an authored one there.
- **Resolution**: Decided with source spans instead (Decision 1), which map the projection's parentheses to the value's source.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: all suites passed.
- [x] The runtime program of `runtime_a_wrapped_concise_arrow_value_runs_in_its_own_async_body` shapes, compiled with `ttc`, type-checked with the repository-pinned TypeScript 7.1 `tsc --strict`, and run with `node`, print the matched and propagated values (`3 0 4 5 1 6` for the match forms; `Ok 2`, `Err neg`, `Ok 6` for the `result` forms).

## Result

Changed `src/program_syntax.rs`, `src/program_syntax/visit.rs`, `src/program_syntax/tests.rs`, `src/codegen/core/planning.rs`, `src/codegen/core/emitter/host.rs`, `tests/compile/cases_11.rs`, `tests/integration.rs`, `docs/ai/tt.md`, `docs/tasks/TASK-455-wrapped-arrow-value-block-body.md`, and `docs/tasks/INDEX.md`. A concise arrow body that wraps its `match` or `result` value now becomes a block body, and the arrow-return rewrite has no IIFE form.
