# TASK-403: Claim let-else whose initializer contains an object literal

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

`function f(n: number) { const Some(value: v) = { kind: "Some" as const, value: n } else { return; }; return v; }` was not claimed as a let-else. The text passed through, and the file failed with `source-not-typescript`: "'const' declarations must be initialized". The initializer scanner `expr_until_else` (`src/parser/lets.rs`) gave up at every top-level `{`, including an object literal that begins the initializer or follows an operator.

## Scope

- Included: The let-else initializer scanner, a shared token rule in `src/parser/cursor.rs` for telling an expression `{` from a block `{`, and the let-else section of docs/ai/tt.md.
- Excluded: `if let`, where the first top-level `{` ends the scrutinee and opens the then-block by design (the Rust form this construct follows). The `try` scanners (`src/parser/tries.rs`) are left unchanged. See Decision 2.

## Decisions

### Decision 1: A `{` begins an expression unless the token before it ends one

- **Context**: In ECMA-262 a `{` where an expression may begin is the start of an ObjectLiteral (PrimaryExpression, 13.2). After `=>` it starts a ConciseBody, and after `as`/`satisfies` or `<` it starts a TypeScript type literal. Each of these is part of the expression. A `{` is a block only after an expression is complete. An ExpressionStatement cannot begin with `{` (14.5), which is why a block can follow a complete expression.
- **Alternatives considered**: Recognising object literals by their contents (`key:` after the brace) would be a heuristic, and it would miss `{}` and shorthand properties. Asking callers to parenthesize the initializer is the workaround the report was about.
- **Decision and rationale**: `cursor::brace_begins_expression` holds when the `{` is the first token of the scan or the token before it cannot end an expression. Tokens that end an expression are a non-keyword identifier (or a dotted property name), `this`/`super`/`null`/`true`/`false`, a string, template, or regex literal, a number, and `)`, `]`, `}`. Operators, `=>`, `?`, `:`, `,`, `(`, `[`, operand-taking keywords (`typeof`, `await`, `new`, `in`, ...), TypeScript type operators (`as`, `satisfies`, `keyof`, ...), and a JSX run all leave an operand to come. `expr_until_else` steps over a `{` that begins an expression as one bracket group, so `:` inside it no longer counts as a ternary colon, and it still aborts on a block `{`.

### Decision 2: The `try` scanners share the old rule, but no input reaches it

- **Context**: `stmt_expr_end` and `unclaimed_try_extent` in `src/parser/tries.rs` also stop at every top-level `{`.
- **Investigation**: `parse_try_tail` claims a statement only when the whole scanned expression is one primary operand (`scan_primary_operand` and `scanner::is_primary_expression`). A primary operand cannot have a top-level `{` in expression position: a leading `{` is a TypeScript `try` block and is excluded by `is_expr_start`, and explicit type arguments (`parse<{ id: number }>(raw)`) are not accepted in a try operand at all, with or without braces. `try parse<number>(raw);` is rejected the same way. A declaration such as `const x = try r ?? { ... };` is already claimed through the `try` expression path. `unclaimed_try_extent` only attributes a failed output self-check to a candidate, and TypeScript reports a `try <expr>` statement at the token right after `try`, which lies before any `{`.
- **Decision and rationale**: No observable defect exists in `try`, so no regression test could fail before a change there. The files are left unchanged, and this record documents why. Explicit type arguments in a `try` operand are a separate limitation and are not part of this task.

## Work log

- 2026-09-27: Reproduced with `ttc -p`: the let-else statement fell through to `source-not-typescript`.
- 2026-09-27: Added `brace_begins_expression`/`ends_expression` to `src/parser/cursor.rs` and used them in `expr_until_else`. Updated that function's doc comment, which described the old "bare `{`" abort.
- 2026-09-27: Examined `tries.rs` as described in Decision 2. Applied the same change there first, found it had no observable effect, and reverted it.
- 2026-09-27: Documented the initializer in the let-else section of docs/ai/tt.md.
- 2026-09-27: Added `let_else_initializer_may_be_an_object_literal` (tests/compile/cases_03.rs) and `runtime_let_else_binds_from_an_object_literal_initializer` (tests/integration/cases_05.rs). Both fail on the previous compiler and pass now.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] `node scripts/check-task-index`

## Result

Changed `src/parser/cursor.rs`, `src/parser/lets.rs`, `docs/ai/tt.md`, `tests/compile/cases_03.rs`, `tests/integration/cases_05.rs`. A let-else initializer may contain object literals in any expression position, including at its start.
