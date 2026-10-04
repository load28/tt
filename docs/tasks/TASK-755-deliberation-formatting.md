# TASK-755: Separate deliberation formatting from execution

- **Status**: In progress
- **Started**: 2026-10-04
- **Completed**: —
- **Commit**: —

## Purpose

Move serialize, untrusted, sharedRules, agentPrompt, mentionPrompt, moderatorPrompt, formatAgentComment and formatOutcome into deliberation-format.mjs. DeliberationEngine imports its five entry points; the two existing public formatters remain re-exported from deliberation.mjs. The class and all awaited operations stay unchanged.

## Scope

- Included: `tools/deliberation-bot/src/deliberation.mjs`, `tools/deliberation-bot/src/deliberation-format.mjs`.
- Excluded: Behavior changes, public API changes, baseline updates, and TASK-732.

## Decisions

### Decision 1: Extract the existing responsibility without redesign

- **Context**: The roadmap identifies a coherent operation mixed with its caller.
- **Alternatives considered**: Keeping the boundary implicit; combining the move
  with algorithm or state redesign, which would make equivalence harder to review.
- **Decision and rationale**: Preserve definitions and callers where possible,
  using the smallest visibility required by the existing consumers. See the
  [detailed brief](../design/refactoring-deliberation-formatting.md).

## Work log

- 2026-10-04: Started from local base `87c69f9d76212a9f129f2cf9f10e8e75a86ad96b` on
  `refactor/remaining-roadmap`. Preserved base source under
  `/tmp/tt-refactor/755` before production edits.

- 2026-10-04: All eight moved definitions and the unchanged engine class were compared with the base. Bot tests passed (4 test files), syntax checks passed, and differential fake-service runs matched every prompt, comment, state, result, and error event across consensus (18), objection (23), continuation (23), and failure (4). No external messages were sent.

## Issues and resolutions

None.

## Regression test (fails before the fix)

Not applicable: Behavior-preserving extraction; no bug is fixed.

## Verification

- [x] Moved definitions and parent changes reviewed.
- [x] Focused checks: Run bot tests/check before and after and compare base/head fake-service event traces for consensus, objections, exhausted rounds, mentions and a request failure. Full npm gates remain required.
- [ ] Applicable complete repository gates.
- [ ] Reference expectations unchanged; task-index and whitespace checks.

## Result

Implementation and focused validation complete; final repository gates remain pending.
