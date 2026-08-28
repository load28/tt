# TASK-267: Preserve typed exhaustiveness when pattern diagnostics are reportable

- **Status**: Pending
- **Started**: 2026-08-28
- **Completed**: —
- **Commit**: —

## Purpose

An unresolved pattern name currently suppresses a match's typed diagnostics
even when the corresponding source-level cause is never emitted. Make
suppression conditional on an owned, reported diagnostic.

## Scope

- Included: typed semantic reporting, per-match ownership, and exhaustiveness
  regression tests
- Excluded: declaration transport and wildcard-based subject identification

## Decisions

### Decision 1: Suppression requires a reported owner

- **Context**: `has_unresolved` is presently sufficient to discard checker and
  coverage consequences.
- **Alternatives considered**: remove suppression; retain implicit suppression;
  or model the emitted cause as the owner.
- **Decision and rationale**: Suppress only when the same pass emits the precise
  source cause. This preserves one-cause diagnostics without creating silence.

## Work log

- 2026-08-28: Created from nightly audit finding 2.

## Issues and resolutions

None.

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`

## Result

Pending.
