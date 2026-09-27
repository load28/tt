# TASK-451: Start a block at a line-broken brace after a complete expression

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: —

## Purpose

A let-else `else` block written without semicolons as `foo`, `Foo`, and `{ return 0; }` on three lines is valid TypeScript whose final block diverges, but ttc reported `let-else-not-diverging`. The same statement splitting judged a match arm block of that shape as falling through, which widened the arm's value type with `undefined`.

## Scope

- Included: Statement splitting in the flow scanner (`src/flow/scanner.rs`, `src/flow/syntax.rs`), which every flow consumer shares: let-else `else` blocks, `if let` bodies and `else` continuations, match arm block bodies, and Result-region outward-control discovery.
- Excluded: The shared `asi_boundary_at` predicate used by the pipeline scans, the parser's expression-head tracking, and the program-syntax projection. Those callers do not know where the current statement starts, so they cannot tell a header's body from a new block, and `{` keeps continuing there as TASK-391 decided. The let-else `;` terminator requirement is unchanged.

## Decisions

### Decision 1: Split before a line-broken `{` after a complete expression unless a head in the statement owes its body

- **Context**: ECMA-262 §12.10.1 inserts a semicolon before a token that no production allows, when a line terminator separates it from the previous token. No expression continues with `{`: after a complete expression, `{` is allowed only as the body of a preceding head (`function f()`, `class A extends B`, `function f(): T`, `interface I extends J`, `namespace N`, `module "m"`, `enum E`, `declare global`). TASK-391 kept `{` as a continuation in `asi_boundary_at` because a single token cannot distinguish `f()` followed by a block from `function f()` followed by its Allman body.
- **Alternatives considered**: Making `asi_boundary_at` split before `{` would break the Allman function-body case (`a_brace_on_its_own_line_does_not_start_a_statement`) and would make the projection write a `;` before a header's body. A context-free walk back from `{` over the head cannot tell a call `foo(x)` from a method or function parameter list.
- **Decision and rationale**: `statement_end` knows where the statement starts, so it asks a statement-aware predicate. `brace_starts_statement` holds when a line terminator separates the top-level `{` from a token that ends an expression (the same precondition `asi_boundary_at` uses, now shared as `line_break_after_expression`) and no head at the statement's top level is still waiting for its body. `head_owes_body` walks the statement once: `function`, `class`, and `enum` push a pending head; `interface`, `namespace`, and `module` push one only when a name or string follows on the same line, which is how TypeScript tells them from identifiers; `global` pushes one after `declare`. A top-level `{` that follows the head keyword itself, a token that ends an expression, or a `>` settles the innermost pending head, because that is where a name, parameter list, heritage, or return type ends. A `{` after an operand-expecting token (`:`, `<`, `|`, `extends`) is a type or object literal inside the head and settles nothing. Heads nest, so `class C extends mixin(function () {})` followed by a line-broken `{` still reads that brace as the class body. The statement-start heads remain decided by `brace_opens_statement`, which runs first.

## Work log

- 2026-09-27: Reproduced the let-else case with `ttc`: `let-else-not-diverging`. A single identifier before the brace (`Foo` then `{ throw 1; }`) fails the same way. A match arm block of the shape compiled with a dead `$tt_v0 = undefined; break;` and a `number | undefined` value slot.
- 2026-09-27: Traced the verdict to `Scanner::statement_end`: `asi_boundary_at` treats `{` as a continuation, and `brace_opens_statement` is false for an expression statement, so `Foo { return 0; }` became one opaque statement.
- 2026-09-27: Added `brace_starts_statement` and `head_owes_body` to `src/flow/syntax.rs`, extracted `line_break_after_expression` from `asi_boundary_at`, and called the new predicate from `statement_end`. The let-else, `if let`, and match arm repros compile; the arm's value slot is `number`.
- 2026-09-27: Added flow unit tests for the split and for every head kind that keeps its Allman body, and an integration test that type-checks and runs the let-else, match arm, and `if let` shapes. Updated the Allman test comment that described the old rule, and noted the refinement at the top of TASK-391.

## Issues and resolutions

### Issue 1: The first integration test draft did not parse

- **Symptom**: `the TypeScript here does not parse: Expected a semicolon` at the let-else.
- **Cause**: The draft omitted the `;` after the let-else `else` block, which the let-else grammar requires (`docs/ai/tt.md`); the failure is independent of this change.
- **Resolution**: Wrote the terminator in the test.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`: all suites passed.
- [x] `a_semicolon_free_brace_after_an_expression_is_a_diverging_block` fails with the previous `src/` (the let-else reports `let-else-not-diverging`) and passes now.

## Result

Changed `src/flow/syntax.rs`, `src/flow/scanner.rs`, `src/flow/tests.rs`, `tests/integration.rs`, `docs/tasks/TASK-391-automatic-semicolon-boundaries.md`, `docs/tasks/TASK-451-brace-after-expression-starts-block.md`, and `docs/tasks/INDEX.md`.
