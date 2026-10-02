# TASK-599: Report a broken `if let` chain once, where it stops

- **Status**: Complete
- **Started**: 2026-09-30
- **Completed**: 2026-09-30
- **Commit**: see `git log --grep TASK-599`

## Purpose

`if let A(n) = o { … } else if let B(s) = o { … } else if (c) { … }`
reported `stray-if-let` twice, at the first and at the second `if`, and
neither at the `else if (c)` that the `if let` grammar rejects (its `else`
is a block or another `if let`, `docs/ai/tt.md`).

## Scope

- Included: the `if let` parser's failure result (`src/parser/iflets.rs`),
  the parser's stray list (`src/parser/parse.rs`, `src/ast.rs`), the
  `stray-if-let` message (`src/sema/checker.rs`), `docs/ai/tt.md`, and
  tests.
- Excluded: what an `if let` accepts is unchanged.

## Decisions

### Decision 1: The parse reports where it stopped, and the chain is one statement

- **Context**: `parse_if_let` returned `Option`, so a failure anywhere in a
  chain was recorded at the outermost `if`. The scanner then resumed after
  that `if`, met the chained `if let` (the `if` after `else`), tried it as
  a statement of its own, and failed it again at its own `if`. The owner
  of the failure is the part of the chain the grammar rejects, and a chain
  is one statement.
- **Alternatives considered**: (a) Suppress the diagnostics of the `if`
  tokens a failed chain contains. That hides a real failure of an inner
  link together with the duplicate. (b) Skip the rest of the chain after a
  failure. Tokens in the bodies would no longer be scanned for their own
  tt constructs. (c) Return the failure's own place and kind from the
  parser, and record each failure once: a malformed link at that link's
  `if` (`StrayIfLetKind::Head`), an `else` that is neither a block nor an
  `if let` at that `else` and the token after it
  (`StrayIfLetKind::ElseContinuation`). A later attempt from an inner link
  of the same chain stops at the same place, so its failure is the same
  one.
- **Decision and rationale**: (c). `parse_if_let` now returns
  `Result<…, StrayIfLet>`; it parses one link (`parse_if_let_link`) and
  then its continuation, where an `if` not followed by `let` is the
  continuation error rather than a failed head. The scanner records a
  failure it has not recorded yet, and sema reports each with a message for
  its kind. The existing recovery nodes are unchanged.

## Work log

- 2026-09-30: Reproduced two diagnostics at the two `if`s for the chain
  above.
- 2026-09-30: `src/ast.rs` (`StrayIfLet`, `StrayIfLetKind`),
  `src/parser/iflets.rs`, `src/parser/parse.rs`, `src/sema/checker.rs`.
- 2026-09-30: Tests: the updated second case of
  `malformed_if_let_is_an_error_with_position` and
  `a_stray_else_of_an_if_let_chain_is_reported_once_where_the_chain_stops`
  (`tests/compile/cases_06.rs`).

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `RUST_TEST_THREADS=2 TTC_REQUIRE_TSGO=1 cargo test`
- [x] `node scripts/check-task-index`

## Result

Changed `src/ast.rs`, `src/parser/iflets.rs`, `src/parser/parse.rs`,
`src/sema/checker.rs`, `docs/ai/tt.md`, `tests/compile/cases_06.rs`,
`docs/tasks/INDEX.md`, and this record.
