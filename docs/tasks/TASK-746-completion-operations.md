# TASK-746: Isolate project completion operations

- **Status**: In progress
- **Started**: 2026-10-04
- **Completed**: —
- **Commit**: —

## Purpose

Move the contiguous completion, triggered_completion, service_completion, pattern_completions, discriminant_candidates, field_candidates, and completion_resolve methods into a private project::completion child with another impl Project. Project and ServiceSession retain all state; children can access the existing private parent methods. Keep source_links, serve, session, and all other operations in project.rs.

## Scope

- Included: `src/engine/language/project.rs`, `src/engine/language/project/completion.rs`.
- Excluded: Behavior changes, public API changes, baseline updates, and TASK-732.

## Decisions

### Decision 1: Extract the existing responsibility without redesign

- **Context**: The roadmap identifies a coherent operation mixed with its caller.
- **Alternatives considered**: Keeping the boundary implicit; combining the move
  with algorithm or state redesign, which would make equivalence harder to review.
- **Decision and rationale**: Preserve definitions and callers where possible,
  using the smallest visibility required by the existing consumers. See the
  [detailed brief](../design/refactoring-completion-operations.md).

## Work log

- 2026-10-04: Started from local base `eb78154e0ee8bdf25eab0f487ce53ddce261332a` on
  `refactor/remaining-roadmap`. Preserved base source under
  `/tmp/tt-refactor/746` before production edits.

- 2026-10-04: Exact moved-method bytes and inverse parent reconstruction passed. cargo fmt --check passed. The native/editor suites exited 0: 173 native and 2 editor tests passed, no failed/ignored/filtered tests; evidence: /tmp/tt-refactor/746/focused.log. Full Rust gates remain required before final completion.

## Issues and resolutions

None.

## Regression test (fails before the fix)

Not applicable: Behavior-preserving extraction; no bug is fixed.

## Verification

- [x] Moved definitions and parent changes reviewed.
- [x] Focused checks: Run cargo fmt --check and the native/editor suites; the preceding TASK-745 full gate provides the unchanged source-built base. The final Rust gate must pass on the complete local stack.
- [ ] Applicable complete repository gates.
- [ ] Reference expectations unchanged; task-index and whitespace checks.

## Result

Implementation and focused validation complete; final repository gates remain pending.
