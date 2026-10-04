# TASK-756: Isolate baseline run selection

- **Status**: In progress
- **Started**: 2026-10-04
- **Completed**: —
- **Commit**: —

## Purpose

Move the pure command-line and case-filter selection predicate to a private helper module, retaining environment reads and baseline I/O in the caller.

## Scope

- Included: `tests/common/baseline.rs`, `tests/common/baseline/selection.rs`.
- Excluded: Behavior changes, public API changes, baseline updates, and TASK-732.

## Decisions

### Decision 1: Extract the existing responsibility without redesign

- **Context**: The roadmap identifies a coherent operation mixed with its caller.
- **Alternatives considered**: Keeping the boundary implicit; combining the move
  with algorithm or state redesign, which would make equivalence harder to review.
- **Decision and rationale**: Preserve definitions and callers where possible,
  using the smallest visibility required by the existing consumers. See the
  [detailed brief](../design/refactoring-baseline-selection.md).

## Work log

- 2026-10-04: Started from local base `8ab26cae865934349c1b1acdbd614bc8b0e73206` on
  `refactor/remaining-roadmap`. Preserved base source under
  `/tmp/tt-refactor/756` before production edits.

- 2026-10-04: The moved predicate is byte-identical and reversing the parent edits reconstructs the base. cargo fmt --check and baseline_tracking passed (2 tests, no ignored/filtered). baseline-tools passed (4 tests, no skips) with the network-enabled subprocess sandbox. The initial restricted run failed because nested spawnSync returned EPERM; a minimal child-process probe reproduced the environment failure without repository code changes.

## Issues and resolutions

The restricted sandbox denied nested Node spawnSync with EPERM, losing child output. The same baseline-tools command passed with network-enabled subprocess permissions; no test expectations or production behavior changed.

## Regression test (fails before the fix)

Not applicable: Behavior-preserving extraction; no bug is fixed.

## Verification

- [x] Moved definitions and parent changes reviewed.
- [x] Focused checks: Run baseline_tracking Rust tests and baseline-tools Node tests; run the complete Rust gate in the final validation.
- [ ] Applicable complete repository gates.
- [ ] Reference expectations unchanged; task-index and whitespace checks.

## Result

Implementation and focused validation complete; final repository gates remain pending.
