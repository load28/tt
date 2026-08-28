# TASK-273: Suppress checker cascades owned by proven tt errors

- **Status**: Pending
- **Started**: 2026-08-28
- **Completed**: —
- **Commit**: —

## Purpose

Some proven tt errors still emit TypeScript consequences from their lowering.
Apply diagnostic ownership consistently so one source cause produces one
actionable report.

## Scope

- Included: non-diverging let-else, tuple arity, ownership ranges, pluralization,
  and typed regressions
- Excluded: unrelated TypeScript errors in the same file or construct body

## Decisions

### Decision 1: Suppression follows explicit lowering ownership

- **Context**: File-level and diagnostic-code suppression can hide independent
  user errors.
- **Alternatives considered**: filter TS2339/TS2367 globally; suppress a whole
  file; or attach cause ownership to emitted glue ranges.
- **Decision and rationale**: Range ownership is the narrow semantic boundary:
  it removes only checker consequences produced by the invalid construct.

## Work log

- 2026-08-28: Created from nightly audit finding 8.

## Issues and resolutions

None.

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`

## Result

Pending.
