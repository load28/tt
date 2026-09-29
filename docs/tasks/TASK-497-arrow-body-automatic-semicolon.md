# TASK-497: Let an automatic semicolon end a block-bodied arrow function before an operator line

- **Status**: Complete
- **Started**: 2026-09-28
- **Completed**: 2026-09-28
- **Commit**: —

## Purpose

Valid TypeScript `export const f = () => {}⏎/x/g.exec("x")` was rejected: `verify-failed: Expected a semicolon`, and, with a tt construct in the file, `source-not-typescript`, which `--no-verify` cannot bypass. `tsc` accepts it. That breaks contract 1 (every valid TypeScript file is a valid `.tt` file).

## Scope

- Included: The vendored SWC parser (`vendor/swc_ecma_parser`), where the rejection comes from, its patch notes, tests at the SWC level and in `tests/passthrough.rs`, a runtime test with a tt construct, and a facts-machine correction the SWC oracle found on the new shapes (Issue 1).
- Excluded: The same-line forms (`() => {} / 2`, `() => {} as any`) and an operator line inside arguments (`g(() => {}⏎/x/g)`), which `tsc` rejects too and still fail.

## Decisions

### Decision 1: Where SWC went wrong, and what TypeScript and ECMA-262 say

- **Context**: ECMA-262 §15.3 makes `ArrowFunction` an `AssignmentExpression`, never a `LeftHandSideExpression` or the left operand of a binary operator. After `() => {}`, a `/`, `+`, `-`, `(`, `[`, or template on the next line cannot continue the arrow function, so it is the offending token of §12.10.1 and, preceded by a line terminator, gets an automatic semicolon. TypeScript's parser returns the arrow function from `parseAssignmentExpressionOrHigher` without looking for an operator, and `parseSemicolon` accepts the line break; the next statement then starts with the `/`, which `parsePrimaryExpression` rescans as a regular expression.
- **Findings**: `parse_paren_expr_or_arrow_fn` (`src/parser/expr.rs`) parses a parenthesized arrow function and, when its body is a block and the current token is a binary operator, reports TS1005 and parses the operator anyway, to mirror TypeScript's error for `() => {} / 2`. It skipped that only for Flow and `<` after a line break. Every other operator after a line break was therefore read as a continuation: `/` as division (a regular expression cannot follow), `+1` and `-1` as binary operators. The identifier-parameter form (`x => {}`) already returns early (`return_if_arrow!`) and was accepted. `(`, `[`, and a template were accepted too: arrow functions return before subscripts are parsed, and the statement then ends at the line break, so SWC already read `() => {}⏎(x)` as two statements, as `tsc` and ECMA-262 do (an arrow function cannot be called without parentheses around it).
- **Alternatives considered**: (a) A lexer-level change so `/` after a `}` on a new line lexes as a regular expression. The lexer does not know which `}` ends an arrow body, and the parser already rescans a `/` as a regular expression at an expression start; the defect is only that the parser took the operator. (b) Rejecting the input with a better message in tt. It is valid TypeScript.
- **Decision and rationale**: Change one condition: parse a binary operator after a block-bodied arrow function only when no line break precedes it (`cur.is_bin_op() && !had_line_break_before_cur()`), which contains the old Flow case. The same-line TS1005 recovery is unchanged. The patch is recorded in `vendor/swc_ecma_parser/TT-PATCH.md`. No SWC issue for this shape was found; acorn had the same defect (acornjs/acorn#475).

### Decision 2: Match `tsc` on the neighbouring shapes

- **Context**: The task asked whether `() => {}⏎(x)`, `⏎[1]`, and a template continue.
- **Decision and rationale**: Checked every shape against `tsc --noEmit` of the pinned TypeScript and against `ttc -p` before and after the patch. `tsc` accepts, and now so does `ttc`: `/x/g`, `/=x/`, `+1`, `-1`, `(1)`, `[1]`, a template, `!x`, `++x` after `() => {}`, `async () => {}`, `(): void => {}`, `<T,>(a: T) => {}`, and an assignment `x = () => {}`. `tsc` rejects, and so does `ttc`: `() => {} / 2`, `() => {} as any`, `() => {}⏎.call(null)`, `⏎?.call(null)`, `⏎as any`, `⏎, 2`, `⏎? 1 : 2`, `⏎< 2`, and `g(() => {}⏎/x/g)`.

## Work log

- 2026-09-28: Reproduced on `13e0e81`: `export const f = () => {}⏎/x/g.exec("x")` fails `verify-failed: Expected a semicolon`; so do `async () => {}`, `(): void => {}`, `<T,>(a: T) => {}` (`Expected '>', got ','`, from the generic-arrow retry), `x = () => {}`, and `⏎+1`/`⏎-1`. Probed the shapes of Decision 2 with `ttc -p` and `tsc`.
- 2026-09-28: Patched `vendor/swc_ecma_parser/src/parser/expr.rs` and `TT-PATCH.md`. Every probe now agrees with `tsc`. With a tt construct, `const f = () => {}⏎/x/g.exec("x") |> console.log` compiles (the lexer's token facts already read the `/` as a regular expression) and runs under node.
- 2026-09-28: Added `tests/swc_arrow_asi.rs` (the parser directly, TypeScript and TSX: the statement count and the second statement's expression kind for each arrow form and each line; the same-line and argument forms still fail), a passthrough test, `an_operator_line_after_a_block_bodied_arrow_function_is_its_own_statement` in `tests/integration.rs`, and two known shapes for the token-facts oracle (Issue 1).

## Issues and resolutions

### Issue 1: The facts machine split an async arrow function with a return type

- **Symptom**: The new oracle shape `const g = async (): Promise<void> => {}` failed `the_machine_reads_known_shapes_as_swc_does`: SWC read one statement, the machine two (`const g = async ()` and `Promise<void> => {}`).
- **Cause**: `async` followed by `(` was read as a name and the `(` as a call's arguments, after which a `:` ends the expression. Only a parenthesized group allows a `:` return type (`after_paren`).
- **Resolution**: `async` followed by `(` on the same line is read as a prefix, like `async` before a name (`src/lexer/facts/expressions.rs`), so the `(` is a group that a return type can follow. A call of a function named `async` reads the same (a group followed by the same tokens), which the second oracle shape covers.

## Verification

- [x] `cargo fmt --check`: exit 0.
- [x] `cargo clippy --all-targets -- -D warnings`: exit 0.
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: exit 0, no failures; no snapshot changed.
- [x] `./scripts/ci extension`: exit 0.
- [x] `scripts/check-task-index`: the index and the records agree.

## Result

Changed `vendor/swc_ecma_parser/src/parser/expr.rs`, `vendor/swc_ecma_parser/TT-PATCH.md`, `src/lexer/facts/expressions.rs`, `src/lexer/facts/tests.rs`, `tests/passthrough.rs`, `tests/integration.rs`, `docs/tasks/INDEX.md`, and this record; added `tests/swc_arrow_asi.rs`.

A block-bodied arrow function followed by a line that starts with an operator, a regular expression, or a bracket now parses the way `tsc` parses it, with or without tt constructs in the file.
