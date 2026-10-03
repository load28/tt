# TASK-739: Check val parameters of functions whose return type has type arguments

- **Status**: Complete
- **Started**: 2026-10-03
- **Completed**: 2026-10-03
- **Commit**: —

## Purpose

A `val` parameter was not read-only, and neither the command line nor the
editor reported `val-mutation`, when its function's return type annotation
carried a type argument list with a comma, such as
`function submit(val order: Order): Result<Order, string> { order.customer = ''; }`.

## Scope

- Included: The return-type skip of the `val` analysis's function body
  recognition, a compiler regression case, and an editor regression case.
- Excluded: Any change to the `val` rules, the language surface, or emitted
  output.

## Decisions

### Decision 1: Step over type argument lists with the lexer's bracket facts

- **Context**: `Checker::body_after_params` (`src/val/checker.rs`) skips a
  return type annotation token by token to find the body. It stepped over
  `(...)`, `[...]`, and object type braces as groups but read the `,` inside
  `Result<Order, string>` as the end of the annotation, so it found no body,
  registered no parameter scope, and judged no mutation in that function.
- **Alternatives considered**: Counting `<` and `>` locally in the skip would
  re-derive what the lexer already decides and would misread a `<` that is not
  a type argument bracket. Asking the host (SWC) parser for each function's
  body range would change how the token-based `val` walk receives its
  structure, which this defect does not require.
- **Decision and rationale**: Treat a `<` that the lexer's facts record as
  opening type arguments or type parameters (`Token::opens_bracket`) as a group
  and continue after its matching `>` (`find_close_at`, the same pair table
  that already steps over `(` and `[`). This applies to every function form the
  walk recognizes (declarations, function expressions, arrows, methods) and to
  any type argument content, including nested lists, function types, and object
  types.

## Work log

- 2026-10-03: Reproduced with `ttc --check`: the user's `submit` reported
  nothing while the same body without a return type reported `val-mutation`.
  Reduced the trigger to `function f(val o: any): T<O, s> { o.c = 1; }`;
  `T<O>`, `Array<number>`, and `{ a: 1 }` return types were reported.
- 2026-10-03: Added `tests/cases/compiler/valParameterIsReadOnlyBehindAGenericReturnType.tt`
  and `tests/cases/editor/valMutationBehindAGenericReturnType.tt`, then the
  fix in `src/val/checker.rs`.

## Issues and resolutions

### Issue 1: No `val-mutation` behind a generic return type

- **Symptom**: `order.customer = ''` in a function declared
  `(val order: Order): Result<Order, string>` produced no diagnostic in
  `ttc --check`, `ttc --out-dir`, or the VS Code adapter.
- **Cause**: The return-type skip in `body_after_params` returned `None` at
  the comma inside the type argument list.
- **Resolution**: Step over lexer-recorded type argument brackets as a group.

## Regression test (fails before the fix)

- **Path**: `tests/cases/compiler/valParameterIsReadOnlyBehindAGenericReturnType.tt`
  (`cargo test --test case_baselines`) and
  `tests/cases/editor/valMutationBehindAGenericReturnType.tt`
  (`cargo test --test editor_cases`)
- **Observed failure**: Without the fix, `ttc --out-dir` reported no
  diagnostic and `ttc --check-types` reported only the `delete` in the arrow
  function, so the `.errors.txt` and `.ts` baselines differed; the editor
  case reported `published: 0 diagnostic(s)` instead of the `val-mutation`
  at `order`.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] Baseline changes reviewed and committed with the change

## Result

Changed `src/val/checker.rs`; added the two regression cases and their
baselines. Every surface now reports `val-mutation` for a mutation through a
`val` parameter regardless of the return type annotation's type arguments.
