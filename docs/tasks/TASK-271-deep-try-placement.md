# TASK-271: Diagnose deep expression `try` placement before verification

- **Status**: Complete
- **Started**: 2026-08-28
- **Completed**: 2026-08-28
- **Commit**: —

## Purpose

A `try` nested inside calls, templates, or object literals can pass through to
TypeScript and become `verify-failed`. Recognize the complete tt construct and
report the placement rule at the source token.

## Scope

- Included: parser recovery for expression positions, `try-placement` spans,
  and deep-expression regressions
- Excluded: changes to valid statement-position `try` lowering

## Decisions

### Decision 1: Parse first, validate placement second

- **Context**: Nesting depth currently changes whether the parser claims
  `try` as tt syntax.
- **Alternatives considered**: revise `verify-failed` wording; scan emitted
  TypeScript; or produce an explicit misplaced-try syntax node.
- **Decision and rationale**: A parser-owned recovery node preserves the tt
  construct and lets semantic placement remain depth-independent.

## Work log

- 2026-08-28: Created from nightly audit finding 6.
- 2026-08-28: Started after TASK-270 moved nested pattern failures into the
  source semantic layer.
- 2026-08-28: Added an expression-boundary parser for misplaced `try` that
  owns the complete operand without requiring a statement semicolon.
- 2026-08-28: Distinguished object-literal value colons and template
  interpolation roots from statement streams while preserving labels,
  switch clauses, and incomplete declaration rollback candidates.
- 2026-08-28: Added source, typed CLI/server, and content-mapper regressions
  for calls, templates, and object values, including exact source spans and
  the absence of verification-layer diagnostics.
- 2026-08-28: Ran formatting, Clippy, and the complete Cargo suite; all
  passed.

## Issues and resolutions

- The first recovery path classified an incomplete `const n = try g()` as an
  expression-placement error. Declaration-equals context is now preserved as
  an unclaimed tt candidate, so the missing-semicolon contract remains
  unchanged.
- Passing a Node test-name option through the extension's npm wrapper placed
  it after the test-file glob and ran the full suite. The accidental run was
  stopped after unrelated installation-path failures and timeouts; the JSON
  server consumer is covered directly by the native regression.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## Result

Deep expression `try` now produces `try-placement` at the complete source
construct before TypeScript verification. Valid statement-position lowering
and incomplete-statement rollback remain unchanged. Changed files:
`src/parser/mod.rs`, `src/parser/tries.rs`, `tests/compile.rs`,
`tests/native.rs`, and `tests/content_mapper.rs`.
