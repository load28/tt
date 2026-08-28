# TASK-271: Diagnose deep expression `try` placement before verification

- **Status**: Pending
- **Started**: 2026-08-28
- **Completed**: —
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

## Issues and resolutions

None.

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`

## Result

Pending.
