# TASK-272: Point malformed `result` tails at the trailing semicolon

- **Status**: Pending
- **Started**: 2026-08-28
- **Completed**: —
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

## Issues and resolutions

None.

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`

## Result

Pending.
