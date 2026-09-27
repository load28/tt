# TASK-391: Honour automatic semicolon insertion around pipelines and statement constructs

> Updated by [TASK-451](./TASK-451-brace-after-expression-starts-block.md): flow statement splitting now ends a statement before a line-broken `{` that follows a complete expression unless a head in the statement still owes its body. Decision 1 still holds for the shared `asi_boundary_at` predicate.

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

Semicolon-free source was compiled differently from how JavaScript reads it. `const a = 1 |> inc` followed by `console.log(a)` on the next line compiled to `const a = inc\nconsole.log(a)(1)` without a diagnostic; `const b = 5` on the next line produced a misleading `stray-pipe`; `const k = 2` followed by `x |> inc` took `2\nx` as the head. A statement `match` after a statement without `;`, and two consecutive statement `match`es, failed with `lowering-plan-failed`.

## Scope

- Included: The shared automatic-semicolon predicate, the pipeline head and step scans, and the program-syntax projection of statement-level constructs.
- Excluded: The `if let`, `let-else`, and `try` statement grammars, which already require their own terminators.

## Decisions

### Decision 1: One automatic-semicolon predicate from the ECMAScript rule

- **Context**: ECMA-262 §12.10.1 inserts a semicolon before a token that the grammar does not allow when a line terminator separates it from the previous token; `++`/`--` are restricted productions. `crate::flow::asi_boundary_at` recognised a boundary only before ten statement keywords, and the pipeline scanners did not use it at all.
- **Alternatives considered**: A pipeline-only line-break rule would give the pipeline and the flow graph two models of one source boundary.
- **Decision and rationale**: The predicate now reports a boundary when the previous token ends an expression, a line terminator follows it, and the next token cannot continue an expression: an identifier other than `in`/`instanceof`, a string, number, regex, JSX run, `!`, `~`, `@`, `#`, or an adjacent `++`/`--`. A template (tagged template), `(`, `[`, `.`, `?.`, operators, `=>`, and `|>` continue. `{` still continues because `function g()` followed by an Allman brace opens the body, which a token cannot distinguish (`a_brace_on_its_own_line_does_not_start_a_statement`). TypeScript does not continue `as`/`satisfies` across a line break, and a type operator word (`as`, `satisfies`, `keyof`, `infer`, `is`, `asserts`, `unique`, `readonly`) before a line break expects its operand, so it is not treated as ending an expression.

### Decision 2: The pipeline scans stop at that boundary

- **Context**: A step ran to `;`, `,`, or a closer; the head started at the last assignment or opener.
- **Decision and rationale**: A step ends at a depth-zero boundary, and the head tracking restarts at a boundary, so a pipeline covers exactly the statement JavaScript reads.

### Decision 3: The projection writes the boundary a placeholder would erase

- **Context**: A statement construct is projected as `(...)`, which joins the previous line into a call when the source relied on automatic semicolon insertion. The concise-arrow case already wrote a `;`, but only for `try` statements.
- **Alternatives considered**: A `;` alone becomes part of the previous statement in the TypeScript AST, and that statement's end then maps to no source byte, so an outer owner was selected and the `match` was hoisted to module scope.
- **Decision and rationale**: Every statement-level construct in a projected body checks for a boundary at its first token. The written `;` is recorded as an `AutomaticSemicolon` segment of zero source width at the construct's start, which is where the source statement ended. A construct that does not begin a token of the flat token stream (inside a template interpolation) is never at a statement boundary.

## Work log

- 2026-09-27: Reproduced each case with `ttc -p`. Generalised `asi_boundary_at` and used it in `src/parser/pipes.rs` and the head tracking in `src/parser/parse.rs`; the pipelines parsed correctly.
- 2026-09-27: Replaced the concise-arrow-only boundary with a statement-level one in `src/program_syntax/projection.rs`. Two consecutive matches then compiled but hoisted the first to module scope; traced to the unmapped statement end and added the zero-width segment.
- 2026-09-27: The full suite exposed the Allman-brace case and a `;` written inside a template interpolation; applied the corrections in Decisions 1 and 3.
- 2026-09-27: Added an integration test that type-checks and runs semicolon-free statements; it fails on the previous compiler and passes now.

## Issues and resolutions

### Issue 1: A pipeline consumed the next line

- **Symptom**: `console.log(a)(1)` at runtime.
- **Cause**: The step scan had no line-terminator boundary.
- **Resolution**: Decisions 1 and 2.

### Issue 2: Statement constructs after an automatic semicolon failed to lower

- **Symptom**: `lowering-plan-failed` at the file's first declaration.
- **Cause**: The projection's `(` continued the previous statement.
- **Resolution**: Decision 3.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: all suites passed.
- [x] `semicolon_free_statements_keep_their_automatic_boundaries` fails with the previous `src/` and passes now.

## Result

Changed `src/flow/syntax.rs`, `src/flow/mod.rs`, `src/parser/pipes.rs`, `src/parser/parse.rs`, `src/program_syntax/projection.rs`, `src/program_syntax/protocol.rs`, and `tests/integration.rs`.
