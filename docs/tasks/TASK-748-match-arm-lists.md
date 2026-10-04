# TASK-748: Isolate match arm list recognition and recovery

- **Status**: In progress
- **Started**: 2026-10-04
- **Completed**: —
- **Commit**: —

## Purpose

Move ArmPart, ArmOutline and its impl, outline_arms, list_arms, parse_whole_arm, parse_arm_list, recover_match_arms, parse_strict_arm_list and arms_tail into private matches::arm_list. Re-export only ArmPart and outline_arms at their original parser visibility; expose recover_match_arms, parse_strict_arm_list and arms_tail to matches. Cursor and pattern/body parsing remain with their existing owners.

## Scope

- Included: `src/parser/matches.rs`, `src/parser/matches/arm_list.rs`.
- Excluded: Behavior changes, public API changes, baseline updates, and TASK-732.

## Decisions

### Decision 1: Extract the existing responsibility without redesign

- **Context**: The roadmap identifies a coherent operation mixed with its caller.
- **Alternatives considered**: Keeping the boundary implicit; combining the move
  with algorithm or state redesign, which would make equivalence harder to review.
- **Decision and rationale**: Preserve definitions and callers where possible,
  using the smallest visibility required by the existing consumers. See the
  [detailed brief](../design/refactoring-match-arm-lists.md).

## Work log

- 2026-10-04: Started from local base `a31466d9fc542f8ddb8cc05d23fe1d7245a3c1fe` on
  `refactor/remaining-roadmap`. Preserved base source under
  `/tmp/tt-refactor/748` before production edits.

- 2026-10-04: Post-format exact-definition and inverse-parent comparison passed. Parser unit tests (12; 407 intentionally filtered), compile (186), editor (2), and passthrough (88) suites passed with zero failures/ignored tests; /tmp/tt-refactor/748/focused.log. No syntax or expected-output changes.

## Issues and resolutions

None.

## Regression test (fails before the fix)

Not applicable: Behavior-preserving extraction; no bug is fixed.

## Verification

- [x] Moved definitions and parent changes reviewed.
- [x] Focused checks: Run parser unit tests plus passthrough, compile and editor_cases tests; final full Rust gate also runs fuzz and compiler baselines.
- [ ] Applicable complete repository gates.
- [ ] Reference expectations unchanged; task-index and whitespace checks.

## Result

Implementation and focused validation complete; final repository gates remain pending.
