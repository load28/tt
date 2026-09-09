# TASK-298: Remove pre-1.0 Result migration compatibility

- **Status**: Complete
- **Started**: 2026-08-30
- **Completed**: 2026-08-30
- **Commit**: See git history.

## Purpose

tt is introduced as a 1.0.0 beta, so the removed Result syntax does not need a
compatibility diagnostic, automatic edit, or one-release migration wording.

## Scope

- Included: remove `<-` Result migration recognition and the temporary crossing
  migration wording, then update tests and public documentation.
- Excluded: changing the new statement-bodied Result syntax or its permanent
  placement diagnostics.

## Decisions

### Decision 1: Treat removed syntax as ordinary passthrough text

- **Context**: a beta introduction has no released users who require a source
  migration path.
- **Alternatives considered**: retain diagnostic edits for one release, or
  remove the compatibility path completely.
- **Decision and rationale**: remove it completely. Valid TypeScript remains
  passthrough; invalid old syntax is left for TypeScript rather than claimed by
  tt as a legacy construct.

## Work log

- 2026-08-30: Started after the beta policy removed the need for migration.
- 2026-08-30: Removed legacy `<-` recognition, its diagnostic code, and its
  automatic edit.
- 2026-08-30: Updated Result fixtures, semantic-token wording, and public
  documentation for the new syntax only.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `./scripts/ci`

## Result

Result blocks support only the statement-bodied `try` syntax. The removed
`<-` form is no longer claimed, diagnosed, or rewritten by tt.
