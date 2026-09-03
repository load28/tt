# TASK-319: Audit valid-program compilation failures

- **Status**: In progress
- **Started**: 2026-09-03
- **Completed**: —
- **Commit**: —

## Purpose

Find valid TypeScript and tt programs that fail to compile, emit invalid
TypeScript, fail typed checking, or violate their runtime semantics. Repair each
confirmed defect in the compiler layer that owns the broken contract.

## Scope

- Included: TypeScript pass-through, tt parsing and lowering, emitted TypeScript,
  typed project checking, source-kind composition, and runtime behavior
- Excluded: New language features, release publication, and external service
  changes

## Decisions

### Decision 1: Treat validity as an end-to-end contract

- **Context**: Parser success alone does not prove that a valid program survives
  lowering, TypeScript parsing, typed checking, and execution.
- **Alternatives considered**: Audit isolated parser cases; rely on the existing
  regression suite; exercise generated and corpus inputs through every applicable
  public compiler boundary.
- **Decision and rationale**: Classify failures by the first responsible boundary
  and preserve each confirmed case as a regression at that boundary.

## Work log

- 2026-09-03: Confirmed the development environment and the complete TASK-318
  gate, then opened a focused follow-up audit for valid-program compilation.

## Issues and resolutions

None.

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`

## Result

In progress.
