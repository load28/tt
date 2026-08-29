# TASK-272: Point malformed `result` tails at the trailing semicolon

- **Status**: Complete
- **Started**: 2026-08-28
- **Completed**: 2026-08-28
- **Commit**: —

## Purpose

A semicolon after a `result` block's final expression currently produces a
generic error at the block start. Preserve the failed tail shape and diagnose
the semicolon with a direct edit.

## Scope

- Included: result-tail parsing, precise source span, suggestion, and snapshots
- Excluded: other malformed result bindings

## Decisions

### Decision 1: Represent the specific malformed tail in parser recovery

- **Context**: The parser presently loses the location that distinguishes this
  error from a generic stray `result`.
- **Alternatives considered**: search backward from the block close; enhance
  the generic message; or retain the tail terminator in the parse result.
- **Decision and rationale**: Retaining syntax evidence gives an exact span and
  avoids punctuation heuristics over arbitrary TypeScript expressions.

## Work log

- 2026-08-28: Created from nightly audit finding 7.
- 2026-08-28: Started after TASK-271 made deep `try` placement parser-owned.
- 2026-08-28: Preserved a final value run's top-level semicolon as a distinct
  parser attempt and emitted `result-tail-semicolon` with whole-block
  ownership and an exact deletion edit.
- 2026-08-28: Kept binding-only blocks and statement tails on the existing
  `stray-result` path, and added full CLI/server snapshots for the new rule.
- 2026-08-28: Made deletion edits render as readable advice without an empty
  code sample while retaining the machine-applicable empty replacement.
- 2026-08-28: Ran formatting, Clippy, and the complete Cargo suite; all
  passed.

## Issues and resolutions

- Storing the parser fact in a new `Program` vector increased the recursive
  AST size enough for Clippy to reject four large enum variants. The specific
  malformed node now travels through the existing parser diagnostic and
  recovery collections, preserving the fact without expanding every program.
- The first rendered deletion advice ended in empty backticks. The shared
  renderer now omits a code sample for empty replacements while the wire edit
  remains unchanged.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## Result

A final value semicolon now produces `result-tail-semicolon` on the exact
punctuation with a deletion edit. Other malformed tails retain the general
result parse diagnostic. Changed files: `src/parser/results.rs`,
`src/parser/mod.rs`, `src/diagnostics.rs`, `src/content_mapper.rs`,
`src/render.rs`, `tests/compile.rs`, and
`tests/fixtures/diagnostic/result-tail-semicolon/`.
