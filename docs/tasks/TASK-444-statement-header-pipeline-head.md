# TASK-444: Start a pipeline head after an `if`/`while`/`for`/`with` header

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

A pipeline used as the unbraced body of a statement with a parenthesized header took the header into its head: `if (c) x |> g;` emitted `if $tt_ap((c) x, g);`, which failed verification, and `for (;;) x |> g;` failed the lowering plan at 1:1. The expected output is `if (c) $tt_ap(x, g);`.

## Scope

- Included: The pipeline-head tracker in `src/parser/parse.rs` (`track_expr_boundary`).
- Excluded: Other constructs whose heads are found by their own scanners.

## Decisions

### Decision 1: Mark the header's opening parenthesis as a statement header on the bracket stack

- **Context**: The tracker saves the enclosing expression's start when it sees `(` and restores it at the matching `)`, so `f(a(b) |> g)` finds `a(b)`. After the keyword `if`, the enclosing start is the `(` itself, so restoring it at `)` made `(c) x` one head. In ECMA-262 the header parentheses are not an expression: `IfStatement : if ( Expression ) Statement`, `IterationStatement : while ( Expression ) Statement`, the `for` / `for-in` / `for-of` / `for await` forms, and `WithStatement : with ( Expression ) Statement` (ECMA-262 §14.6, §14.7, §14.11). The token after the closing `)` begins the `Statement`, which TypeScript's parser also parses with `parseStatement()` after `parseExpected(CloseParenToken)`.
- **Alternatives considered**: Resetting the head on every `)` followed by an identifier would break the call and grouping cases (`(c) |> g`, `f(a) |> g`) and is a token-shape heuristic. Special-casing the lowering plan would hide the parse error rather than fix it.
- **Decision and rationale**: When the `(` immediately follows an undotted `if`, `while`, `for`, `with`, or `for await`, the tracker pushes a `StatementHeader` frame instead of the saved start; the matching `)` then starts a fresh expression, as the grammar says. A dotted keyword (`o.if (...)`) is a member name and keeps the ordinary call behaviour. `switch` and `catch` headers are always followed by `{`, which already starts fresh.

## Work log

- 2026-09-27: Reproduced verify-failed on `if (c) x |> g;` and `while (c) x |> g;`, and lowering-plan-failed on `for (;;) x |> g;`.
- 2026-09-27: Replaced the `(usize, bool)` bracket stack with an `ExprFrame` enum carrying `Resume` or `StatementHeader`, and added `opens_statement_header`.
- 2026-09-27: Added `a_pipeline_as_the_unbraced_body_of_a_statement_header_starts_after_the_header` to `tests/compile/cases_11.rs`, covering `if`, `if ... else`, `while`, `for (;;)`, `for ... of`, `for await`, and a parenthesized head `(c) |> g`; it fails without the parser change.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`

## Result

Changed `src/parser/parse.rs`, `tests/compile/cases_11.rs`, and `docs/tasks/INDEX.md`.
