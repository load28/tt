# TASK-758: Preserve the Project API layout after completion extraction

- **Status**: In progress
- **Started**: 2026-10-04
- **Completed**: —
- **Commit**: —

## Purpose

Fix PR #139's public Rust API baseline failure caused by TASK-746 moving public
completion methods into a separate inherent impl block.

## Scope

- Included: Project completion API declarations and their private implementation
  boundary, task records, and refactoring audit corrections.
- Excluded: Completion behavior, API signatures, reference baselines, coverage
  thresholds, test normalization, dependency changes, and TASK-732.

## Decisions

### Decision 1: Preserve public declarations and delegate to private operations

- **Context**: Rustdoc lists inherent impl blocks separately and preserves
  declaration order. The API contract test observes that layout.
- **Alternatives considered**: Accept a new API baseline; normalize or hide impl
  blocks in the test; undo the extraction entirely.
- **Decision and rationale**: Restore the original four public declarations,
  documentation and order in the existing Project impl. Keep completion
  implementations in the private child behind narrow internal methods. Preserve
  every operation body and state mutation order; add no public method or attribute
  that hides an API change.

## Work log

- 2026-10-04: Doctor passed; no setup was rerun. Read the failure in coverage job
  111445388558 of CI run 37205386711 at head
  834e4d682ee392b8cd39c53d4d4bbe6d09c6d0dd. The failing test is
  `the_rust_api_matches_its_baseline`, not the coverage percentage threshold.
  The native/extension, diagnostic delta, and performance jobs had passed.

- 2026-10-04: Restored the four public declarations in their original order.
  The existing API regression test now passes (one selected test; two intentionally
  filtered). Compared all six private operation bodies with the pre-fix child,
  and all four public declarations/docs with main; each comparison matched.
  Formatting and clippy with warnings denied passed. No baseline or test changed.
- 2026-10-04: The serial native/public-api validation attempt again exhausted
  container resources and stalled during the native suite. Stopped only its
  process tree; the log is `/tmp/tt-refactor/758/checks.log`. This attempt is not
  a passing native suite. The exact Rust API regression passed separately.

## Issues and resolutions

The existing container has more than 32,000 unreaped processes. Keep validation
serial and record any resource failures separately from the regression.

## Regression test (fails before the fix)

- **Path**: `tests/public_api.rs::the_rust_api_matches_its_baseline`.
- **Observed failure**: CI reports an extra `impl Project` header and completion
  methods appearing before hover/definition/references in
  `tests/baselines/reference/api/ttc.api.txt`. The unchanged test also failed locally (exit 101) before edits with the same
  extra impl block and reordered declarations; log: `/tmp/tt-refactor/758/before.log`.

## Verification

- [x] Reproduce the unchanged test's failure before production edits.
- [x] Public API test passes without changing its baseline or implementation.
- [x] Completion bodies and public declarations compared with original sources.
- [ ] Rust formatting, clippy, and applicable tests.
- [ ] Task-index and whitespace checks; same PR updated.

## Result

Implementation and validation in progress.
