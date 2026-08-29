# TASK-273: Suppress checker cascades owned by proven tt errors

- **Status**: Complete
- **Started**: 2026-08-28
- **Completed**: 2026-08-28
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
- 2026-08-28: Started after TASK-272 preserved precise parser ownership for
  malformed result tails.
- 2026-08-28: Attached let-else head ownership to non-divergence causes and
  tuple-match head ownership to arity causes, matching the anchors emitted by
  their lowering paths.
- 2026-08-28: Centralized tuple-match head construction on the AST so HIR
  lowering and semantic diagnostics cannot drift to different owner ranges.
- 2026-08-28: Added typed CLI/server regressions proving TS2339 and TS2367
  consequences disappear while an independent TS2322 in the same file stays.
- 2026-08-28: Corrected singular `element` and `scrutinee` rendering and ran
  formatting, Clippy, and the complete Cargo suite; all passed.

## Issues and resolutions

None.

## Verification

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`

## Result

Non-diverging let-else and tuple-arity causes now own the exact lowering anchor
that produces their checker cascades. The shared range filter removes those
consequences without hiding source-mapped TypeScript errors. Changed files:
`src/ast.rs`, `src/hir/lower.rs`, `src/sema.rs`, `tests/compile.rs`, and
`tests/native.rs`.
