# TASK-448: Project an `if let` in an unbraced body as one statement

- **Status**: Complete
- **Started**: 2026-09-27
- **Completed**: 2026-09-27
- **Commit**: see `git log --grep TASK-448`

## Purpose

An `if let` written as the unbraced body of an `if` or loop was reported as
`source-not-typescript` although the source is valid tt:
`for (const x of xs) if let Some(value) = x { ... } else { break; }` gave
"A 'break' statement can only be used within an enclosing iteration
statement", and `if (c) if let ... { } else { }` followed by `else { }` gave
"Expression expected".

## Scope

- Included: the TypeScript projection of statement decisions (`if let`,
  `let-else`) in `src/program_syntax/projection.rs`; compile and runtime
  regression tests.
- Excluded: emitted TypeScript, which already put the lowered `if let` in its
  own block.

## Decisions

### Decision 1: Wrap the whole statement-decision projection in one block

- **Context**: `emit_statement_decision` projected a placeholder statement
  `{$tt_syntax_stmt_N;}` and then, separately, `if (true) {...} else {...}`
  for the bodies. Under an unbraced parent only the placeholder became the
  body; the bodies escaped the loop (so `break` had no target) and an outer
  `else` then followed a complete statement.
- **Alternatives considered**: (a) folding the placeholder into the
  condition (`if ($tt_syntax_stmt_N) ...`) — changes the placeholder's
  syntax category and every consumer that reads it; (b) wrapping only when
  the parent is unbraced — the projection has no parent context, and a
  block is harmless in every other position.
- **Decision and rationale**: The source decision is one statement, so its
  projection is one block `{ {$tt_syntax_stmt_N;} if (true) {...} else {...} }`.
  The if-let's own `else` is still inside the block, so an `else` after it in
  the source binds to the outer `if` exactly as the tt parser bound it
  (ECMA-262 §14.6: an `else` pairs with the nearest `if` that lacks one,
  which the tt parser applies when it takes the if-let's own `else`). A block
  is neither an iteration statement nor a label target, so `break`,
  `continue label`, and `return` inside keep their targets.

## Work log

- 2026-09-27: Reproduced both reports with `ttc -p`; confirmed the emitted
  TypeScript is already braced (`for (...) { const $tt_t0 = x; if (...) ... }`).
- 2026-09-27: `src/program_syntax/projection.rs`: `emit_statement_decision`
  opens and closes one block around the placeholder and the bodies.
- 2026-09-27: Tests `an_if_let_as_an_unbraced_body_is_projected_as_one_statement`
  (`tests/compile/cases_11.rs`, including a labeled loop and an
  `else if let` chain under `while`) and
  `runtime_an_if_let_as_an_unbraced_body_keeps_its_parent_and_its_else`
  (`tests/integration/cases_05.rs`, tsc + node).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `TTC_REQUIRE_TSGO=1 cargo test`
- [x] Runtime: `f` stops at the first `None` (3), `g` gives `1 2 3`, `h` gives
  `1 3 4` (the next-line `else` binds to the if-let), and `continue outer`
  reaches the labeled loop (3).

## Result

Changed `src/program_syntax/projection.rs`, `tests/compile/cases_11.rs`,
`tests/integration/cases_05.rs`, `docs/tasks/INDEX.md`, and this record.
