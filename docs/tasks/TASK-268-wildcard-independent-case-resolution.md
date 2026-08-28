# TASK-268: Diagnose unknown cases independently of wildcard coverage

- **Status**: Pending
- **Started**: 2026-08-28
- **Completed**: —
- **Commit**: —

## Purpose

Unknown case resolution currently depends on whether other arms identify the
variant and therefore changes when a wildcard is added. Resolve case names
from the scrutinee's authoritative domain independently of coverage.

## Scope

- Included: case-name resolution with and without `_`, batch and typed parity,
  server and content-mapper regressions
- Excluded: field transport and exhaustiveness suppression ownership

## Decisions

### Decision 1: Name resolution and coverage remain separate compiler questions

- **Context**: A wildcard changes coverage but must not change whether a named
  constructor exists.
- **Alternatives considered**: special-case wildcard matches; rely on TS2678;
  or supply the resolver with the authoritative subject.
- **Decision and rationale**: Resolve against the subject domain and run
  coverage afterward. Wildcard-specific branches would encode syntax shape
  instead of language semantics.

## Work log

- 2026-08-28: Created from nightly audit finding 3.

## Issues and resolutions

None.

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`

## Result

Pending.
